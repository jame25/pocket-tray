//! TTS Engine wrapper - handles model loading and audio generation

use crate::settings::{embedded_config, models_dir, VOICES};
use anyhow::Result;
use pocket_tts::{ModelState, TTSModel};
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;

/// Commands sent to the TTS thread
#[derive(Debug)]
pub enum TTSCommand {
    Speak { text: String },
    Stop,
    ChangeVoice { voice: String },
    Shutdown,
}

/// Events sent from the TTS thread
#[derive(Debug)]
pub enum TTSEvent {
    ModelLoaded,
    StartedSpeaking,
    FinishedSpeaking,
    Error(String),
}

/// TTS Engine running in a dedicated thread
pub struct TTSEngine {
    model: TTSModel,
    voice_states: HashMap<String, ModelState>,
    current_voice: String,
    is_speaking: Arc<AtomicBool>,
    cmd_rx: Receiver<TTSCommand>,
    event_tx: Sender<TTSEvent>,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
}

impl TTSEngine {
    /// Create a new TTS engine
    pub fn new(
        initial_voice: &str,
        is_speaking: Arc<AtomicBool>,
        cmd_rx: Receiver<TTSCommand>,
        event_tx: Sender<TTSEvent>,
    ) -> Result<Self> {
        let models_path = models_dir()?;

        // Verify models directory exists
        if !models_path.exists() {
            anyhow::bail!(
                "Models directory not found at: {}. Please place the models folder next to the executable.",
                models_path.display()
            );
        }

        let weights_path = models_path.join("tts_b6369a24.safetensors");
        let tokenizer_path = models_path.join("tokenizer.model");

        // Verify required files exist
        if !weights_path.exists() {
            anyhow::bail!("Model weights not found at: {}", weights_path.display());
        }
        if !tokenizer_path.exists() {
            anyhow::bail!("Tokenizer not found at: {}", tokenizer_path.display());
        }

        log::info!("Loading TTS model from: {}", models_path.display());

        // Load model using offline method
        let config = embedded_config();
        let model = TTSModel::load_offline(&weights_path, &tokenizer_path, config)?;

        log::info!("Model loaded successfully");

        // Pre-load all voice states
        let mut voice_states = HashMap::new();
        for voice_name in VOICES {
            let voice_path = models_path.join(format!("{}.safetensors", voice_name));
            if voice_path.exists() {
                match model.get_voice_state_from_prompt_file(&voice_path) {
                    Ok(state) => {
                        log::info!("Loaded voice: {}", voice_name);
                        voice_states.insert(voice_name.to_string(), state);
                    }
                    Err(e) => {
                        log::warn!("Failed to load voice '{}': {}", voice_name, e);
                    }
                }
            } else {
                log::warn!("Voice file not found: {}", voice_path.display());
            }
        }

        if voice_states.is_empty() {
            anyhow::bail!("No voice files found in models directory");
        }

        // Initialize audio output
        let (_stream, stream_handle) = OutputStream::try_default()?;

        // Use initial voice if available, otherwise use first available
        let current_voice = if voice_states.contains_key(initial_voice) {
            initial_voice.to_string()
        } else {
            voice_states.keys().next().unwrap().clone()
        };

        log::info!("Using voice: {}", current_voice);

        Ok(Self {
            model,
            voice_states,
            current_voice,
            is_speaking,
            cmd_rx,
            event_tx,
            _stream,
            stream_handle,
        })
    }

    /// Run the TTS engine loop
    pub fn run(&mut self) {
        // Notify that model is loaded
        let _ = self.event_tx.send(TTSEvent::ModelLoaded);

        loop {
            match self.cmd_rx.recv() {
                Ok(TTSCommand::Speak { text }) => {
                    self.speak(&text);
                }
                Ok(TTSCommand::Stop) => {
                    self.is_speaking.store(false, Ordering::SeqCst);
                }
                Ok(TTSCommand::ChangeVoice { voice }) => {
                    if self.voice_states.contains_key(&voice) {
                        self.current_voice = voice;
                        log::info!("Voice changed to: {}", self.current_voice);
                    } else {
                        log::warn!("Voice '{}' not available", voice);
                    }
                }
                Ok(TTSCommand::Shutdown) | Err(_) => {
                    log::info!("TTS engine shutting down");
                    break;
                }
            }
        }
    }

    /// Speak the given text
    fn speak(&mut self, text: &str) {
        let voice_state = match self.voice_states.get(&self.current_voice) {
            Some(s) => s,
            None => {
                let _ = self.event_tx.send(TTSEvent::Error(format!(
                    "Voice '{}' not loaded",
                    self.current_voice
                )));
                return;
            }
        };

        // Create a new sink for this speech
        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(e) => {
                let _ = self.event_tx.send(TTSEvent::Error(format!("Audio error: {}", e)));
                return;
            }
        };

        self.is_speaking.store(true, Ordering::SeqCst);
        let _ = self.event_tx.send(TTSEvent::StartedSpeaking);

        log::info!("Speaking: {}", text);

        // Stream generation
        for chunk_result in self.model.generate_stream_long(text, voice_state) {
            // Check for stop command (non-blocking)
            match self.cmd_rx.try_recv() {
                Ok(TTSCommand::Stop) => {
                    log::info!("Speech stopped by user");
                    sink.stop();
                    break;
                }
                Ok(TTSCommand::Shutdown) => {
                    sink.stop();
                    self.is_speaking.store(false, Ordering::SeqCst);
                    return;
                }
                Ok(TTSCommand::ChangeVoice { voice }) => {
                    if self.voice_states.contains_key(&voice) {
                        self.current_voice = voice;
                    }
                }
                Ok(TTSCommand::Speak { .. }) => {
                    // Ignore new speech requests while speaking
                }
                Err(TryRecvError::Empty) => {
                    // No command, continue
                }
                Err(TryRecvError::Disconnected) => {
                    sink.stop();
                    self.is_speaking.store(false, Ordering::SeqCst);
                    return;
                }
            }

            match chunk_result {
                Ok(chunk) => {
                    // Convert tensor to samples
                    let samples = self.tensor_to_samples(&chunk);
                    if !samples.is_empty() {
                        let buffer = SamplesBuffer::new(
                            1,                                   // channels
                            self.model.sample_rate as u32,       // 24000
                            samples,
                        );
                        sink.append(buffer);
                    }
                }
                Err(e) => {
                    log::error!("TTS generation error: {}", e);
                    let _ = self.event_tx.send(TTSEvent::Error(format!("Generation error: {}", e)));
                    break;
                }
            }
        }

        // Wait for playback to finish (unless stopped)
        if self.is_speaking.load(Ordering::SeqCst) {
            sink.sleep_until_end();
        }

        self.is_speaking.store(false, Ordering::SeqCst);
        let _ = self.event_tx.send(TTSEvent::FinishedSpeaking);
        log::info!("Speech finished");
    }

    /// Convert a tensor to f32 samples
    fn tensor_to_samples(&self, tensor: &candle_core::Tensor) -> Vec<f32> {
        // Tensor is [B, C, T] - squeeze batch and channel to get [T]
        let squeezed = tensor
            .squeeze(0)
            .and_then(|t| t.squeeze(0))
            .unwrap_or_else(|_| tensor.clone());

        squeezed.to_vec1::<f32>().unwrap_or_default()
    }
}

/// Spawn the TTS engine in a separate thread
pub fn spawn_tts_thread(
    initial_voice: String,
    is_speaking: Arc<AtomicBool>,
    cmd_rx: Receiver<TTSCommand>,
    event_tx: Sender<TTSEvent>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("tts-engine".into())
        .spawn(move || {
            match TTSEngine::new(&initial_voice, is_speaking, cmd_rx, event_tx.clone()) {
                Ok(mut engine) => {
                    engine.run();
                }
                Err(e) => {
                    log::error!("Failed to initialize TTS engine: {}", e);
                    let _ = event_tx.send(TTSEvent::Error(format!("Init failed: {}", e)));
                }
            }
        })
        .expect("Failed to spawn TTS thread")
}
