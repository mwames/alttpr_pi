use std::sync::Mutex;

use sdl3::{AudioSubsystem, Sdl};
use sdl3::audio::{AudioCallback, AudioDevice, AudioSpec, AudioStreamWithCallback};

pub const SPEC: AudioSpec = AudioSpec {
    freq: Some(32000),
    channels: Some(2),
    format: Some(sdl3::audio::AudioFormat::S16LE),
};

static mut AUDIO_BUFFER: Mutex<Vec<i16>> = Mutex::new(Vec::new());

// This function will be called by the libretro core to provide audio samples.
// We will store these samples in a global buffer, which we can then use to feed the SDL audio subsystem.
pub unsafe extern "C" fn retro_audio_sample_batch(data: *const i16, frames: usize) -> usize {
    // Check for a null pointer, which shouldn't normally happen.
    if data.is_null() {
        return 0;
    }
    
    // Each frame is 2 samples (stereo: left and right)
    let total_samples = frames * 2;
    let samples: &[i16] = unsafe { 
        std::slice::from_raw_parts(data, total_samples)
    };
    
    // Process the samples:
    // For instance, you might write these samples into an audio ring buffer,
    // send them to SDL's audio subsystem, or convert them into another format.
    AUDIO_BUFFER.lock().unwrap().extend_from_slice(samples);

    // Return the number of frames processed.
    // If you've processed them all, simply return `frames`.
    frames
}

// Audio Callback
pub struct AudioHandler;
impl AudioCallback<i16> for AudioHandler {
    fn callback(&mut self, out: &mut [i16]) {
        unsafe {
            // Lock the buffer.
            let mut audio_buffer = AUDIO_BUFFER.lock().unwrap();
            // Determine how many samples we can copy.
            let samples_to_copy = out.len().min(audio_buffer.len());
    
            if samples_to_copy > 0 {
                // Copy available samples into the output.
                out[..samples_to_copy].copy_from_slice(&audio_buffer[..samples_to_copy]);
                // Remove only the samples that have been consumed.
                audio_buffer.drain(..samples_to_copy);
            }
        }
    }
}

pub fn initialize_audio_subsystem(sdl_context: &Sdl) -> Result<(AudioSubsystem, AudioDevice, AudioStreamWithCallback<AudioHandler>), String> {
    // Initialize the audio subsystem
    let audio_subsystem = sdl_context.audio().unwrap();
    let playback_device = audio_subsystem.default_playback_device();
    
    let callback = AudioHandler;
    let playback_stream = match playback_device.open_playback_stream_with_callback(&SPEC, callback) {
        Ok(stream) => stream,
        Err(e) => panic!("Failed to open playback stream: {}", e),
    };
    // Start the audio device
    playback_stream.resume().unwrap();

    Ok((audio_subsystem, playback_device, playback_stream))
}