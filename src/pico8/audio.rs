//! Shows how to create a custom [`Decodable`] type by implementing a Sine wave.

use bevy::{
    audio::{AddAudioSource, AudioPlugin, Source},
    math::ops,
    prelude::*,
    reflect::TypePath,
    utils::Duration,
};
use dasp::{signal, Sample, Signal};
use std::sync::Arc;
use std::f32;

const SAMPLE_RATE: u32 = 22_050;

enum WaveForm {
    Sine,
    Triangle,
    Sawtooth,
    LongSquare,
    ShortSquare,
    Ringing,
    Noise,
    RingingSine,
    Custom(u8)
}

impl From<WaveForm> for u8 {
    fn from(wave: WaveForm) -> u8 {
        use WaveForm::*;
        match wave {
            Sine => 0,
            Triangle => 1,
            Sawtooth => 2,
            LongSquare => 3,
            ShortSquare => 4,
            Ringing => 5,
            Noise => 6,
            RingingSine => 7,
            Custom(x) => x + 8
        }
    }
}

impl From<u8> for WaveForm {
    fn from(value: u8) -> WaveForm {
        use WaveForm::*;
        match value {
            0 => Sine,
            1 => Triangle,
            2 => Sawtooth,
            3 => LongSquare,
            4 => ShortSquare,
            5 => Ringing,
            6 => Noise,
            7 => RingingSine,
            x => Custom(x as u8 - 8)
        }
    }
}

impl From<u8> for Effect {
    fn from(value: u8) -> Effect {
        use Effect::*;
        match value {
            0 => None,
            1 => Slide,
            2 => Vibrato,
            3 => Drop,
            4 => FadeIn,
            5 => FadeOut,
            6 => ArpFast,
            7 => ArpSlow,
            x => panic!("Unexpected effect {x}")
        }
    }
}

impl From<Effect> for u8 {
    fn from(value: Effect) -> u8 {
        use Effect::*;
        match value {
            None => 0,
            Slide => 1,
            Vibrato => 2,
            Drop => 3,
            FadeIn => 4,
            FadeOut => 5,
            ArpFast => 6,
            ArpSlow => 7,
        }
    }
}

enum Effect {
    // 0 none, 1 slide, 2 vibrato, 3 drop, 4 fade_in, 5 fade_out, 6 arp fast, 7
 // arp slow; arpeggio commands loop over groups of four notes at speed 2 (fast)
 // and 4 (slow)
    None,
    Slide,
    Vibrato,
    Drop,
    FadeIn,
    FadeOut,
    ArpFast,
    ArpSlow
}

trait Note {
    /// This is the pitch in midi format [0, 127].
    fn pitch(&self) -> u8;
    fn wave(&self) -> WaveForm;
    /// The volume [0, 1]
    fn volume(&self) -> f32;
    fn effect(&self) -> Effect;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pico8Note(u16);


impl Pico8Note {
    fn new(pitch: u8,
           volume: f32,
           wave: WaveForm,
           effect: Effect) -> Self {
        Pico8Note(
            (pitch & 0b0011_1111) as u16 |
            u8::from(wave) as u16 & 0b111 << 6 |
            (volume * 7.0) as u16 & 0b111 << 9 |
            u8::from(effect) as u16 & 0b111 << 12)
    }
}

// impl From<u8> for Pico8Note {
//     fn from(value: u8) -> Self {
//         Pico8Note::new(value, 5.0 / 7.0, WaveForm::Sine, Effect::None)
//     }

// }

impl From<u16> for Pico8Note {
    fn from(value: u16) -> Self {
        Pico8Note(value)
    }
}

impl From<Pico8Note> for u16 {
    fn from(value: Pico8Note) -> Self {
        value.0
    }
}

impl Default for Pico8Note {
    fn default() -> Self {
        Pico8Note::new(32, 5.0 / 7.0, WaveForm::Sine, Effect::None)
    }
}

impl Note for Pico8Note {
    fn pitch(&self) -> u8 {
        (self.0 & 0b0011_1111) as u8 + 36
    }

    fn wave(&self) -> WaveForm {
        WaveForm::from((self.0 >> 6 & 0b111) as u8)
    }

    fn volume(&self) -> f32 {
        (self.0 >> 9 & 0b111) as f32 / 7.0
    }

    fn effect(&self) -> Effect {
        Effect::from((self.0 >> 12 & 0b111) as u8)
    }
}


// This struct usually contains the data for the audio being played.
// This is where data read from an audio file would be stored, for example.
// This allows the type to be registered as an asset.
#[derive(Asset, TypePath, Clone, Default)]
struct Sfx {
    notes: Vec<Pico8Note>,
    speed: u8,
}

impl Sfx {
    fn new(notes: impl IntoIterator<Item = Pico8Note>) -> Self {
        Sfx {
            notes: notes.into_iter().collect(),
            speed: 16
        }
    }

    fn with_speed(mut self, speed: u8) -> Self {
        self.speed = speed;
        self
    }
}

impl SfxDecoder {
    fn new(sample_rate: u32, speed: u8, duration: Option<Duration>, notes: Vec<Pico8Note>) -> Self {
        Self {
            sample_rate,
            speed,
            duration,
            notes: notes.into_iter(),
            samples: None,
        }
    }
}

struct SfxDecoder {
    sample_rate: u32,
    speed: u8,
    duration: Option<Duration>,
    notes: std::vec::IntoIter<Pico8Note>,
    samples: Option<Box<dyn Iterator<Item = f32> + Sync + Send + 'static>>,
}

impl Iterator for SfxDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples.is_none() {
            self.samples =
            self.notes.next().map(|note| {
                let freq = 440.0 * f32::exp2((note.pitch() as i8 - 69) as f32/12.0);
                let hz = signal::rate(self.sample_rate as f64).const_hz(freq as f64);
                let duration = (self.speed as f32 / 120.0) * self.sample_rate as f32;
                let mut synth = hz
                    .clone()
                    .sine()
                    .map(|x| x as f32);
                Box::new(synth.take(duration as usize)) as Box<dyn Iterator<Item = f32> + Sync + Send + 'static>
            });

        }
        self.samples.as_mut().and_then(|samples| samples.next())
    }
}

impl Source for SfxDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.duration.clone()
    }
}

impl Decodable for Sfx {
    type DecoderItem = <SfxDecoder as Iterator>::Item;

    type Decoder = SfxDecoder;

    fn decoder(&self) -> Self::Decoder {
        let sample_rate = SAMPLE_RATE;
        let duration = Some(Duration::from_secs_f32(self.notes.len() as f32 * 183.0 / sample_rate as f32));
        SfxDecoder::new(
            sample_rate,
            self.speed,
            duration,
            self.notes.clone(),
        )
    }
}

// fn main() {
//     let mut app = App::new();
//     // register the audio source so that it can be used
//     app.add_plugins(DefaultPlugins.set(AudioPlugin {
//         global_volume: GlobalVolume::new(0.2),
//         ..default()
//     }))
//     .add_audio_source::<Sfx>()
//     .add_systems(Startup, setup)
//     .run();
// }

// fn setup(mut assets: ResMut<Assets<Sfx>>, mut commands: Commands) {
//         // .take(duration)
//         // .chain(hz.clone().saw().take(duration))
//         // .chain(hz.clone().square().take(duration))
//         // .chain(hz.clone().noise_simplex().take(duration))
//         // .chain(signal::noise(0).take(duration))
//         // .map(|s| s.to_sample::<f32>() * 0.2)
//         ;
//     // add a `Sfx` to the asset server so that it can be played
//     let audio_handle = assets.add(Sfx::new([Pico8Note::default()])
//     // .with_speed(128)
//     );//Sfx::new(synth));//  {
//     //     // frequency: 440., // this is the frequency of A4
//     //     signal: Box::new(synth), // this is the frequency of A4
//     // });
//     commands.spawn(AudioPlayer(audio_handle));
// }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_flat_map() {
        let a = 0..3;
        let b = 3..6;
        let c = 6..9;
        let v = vec![a, b, c];
        v.iter().flat_map(|it| it.clone());
    }

    #[test]
    fn test_flat_map2() {
        let a = 0..3;
        let b = 3..6;
        let c = 6..9;
        let v = vec![a, b, c];
        let w = v.into_iter().flat_map(|it| it);
        assert_eq!((0..9).collect::<Vec<_>>(), w.collect::<Vec<_>>());
    }

    #[test]
    fn check_note_conversion() {
        let a = Pico8Note::default();
        let x = u16::from(a);
        let b = Pico8Note::from(x);
        assert_eq!(a, b);
    }
}
