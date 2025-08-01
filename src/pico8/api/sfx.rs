use super::*;
use bevy::audio::PlaybackMode;

use crate::pico8::audio::{AudioCommand, SfxDest};

#[derive(Debug, Clone, Copy)]
pub enum SfxCommand {
    Play(u8),
    Release,
    Stop,
}

impl From<u8> for SfxCommand {
    fn from(x: u8) -> Self {
        SfxCommand::Play(x)
    }
}

impl super::Pico8<'_, '_> {
    // sfx( n, [channel,] [offset,] [length] )
    pub fn sfx(
        &mut self,
        n: impl Into<SfxCommand>,
        channel: Option<u8>,
        offset: Option<u8>,
        length: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error> {
        assert!(offset.is_none(), "offset not implemented");
        assert!(length.is_none(), "length not implemented");
        let n = n.into();
        let bank = bank.unwrap_or(0);
        match n {
            SfxCommand::Release => {
                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands
                        .queue(AudioCommand::Release(SfxDest::Channel(chan)));
                } else {
                    self.commands.queue(AudioCommand::Release(SfxDest::Any));
                }
            }
            SfxCommand::Stop => {
                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands.queue(AudioCommand::Stop(
                        SfxDest::Channel(chan),
                        Some(PlaybackMode::Remove),
                    ));
                } else {
                    self.commands
                        .queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Remove)));
                }
            }
            SfxCommand::Play(n) => {
                let sfx = self
                    .pico8_asset()?
                    .audio_banks
                    .get(bank as usize)
                    .ok_or(Error::NoAsset(format!("bank {bank}").into()))?
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("sfx {n}").into()))?
                    .clone();

                if let Some(chan) = channel {
                    // let chan = self.sfx_channels[chan as usize];
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Channel(chan),
                        PlaybackSettings::REMOVE,
                    ));
                } else {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Any,
                        PlaybackSettings::REMOVE,
                    ));
                }
            }
        }
        Ok(())
    }

    // music( n, [facems,] [channelmask,] )
    pub fn music(
        &mut self,
        n: impl Into<SfxCommand>,
        _fade_ms: Option<u32>,
        channel_mask: Option<u8>,
        bank: Option<u8>,
    ) -> Result<(), Error> {
        let n = n.into();
        let bank = bank.unwrap_or(0);
        match n {
            SfxCommand::Release => {
                panic!("Music does not accept a release command.");
            }
            SfxCommand::Stop => {
                // if let Some(chan) = channel {
                //     let chan = self.sfx_channels[chan as usize];
                //     self.commands
                //         .queue(AudioCommand::Stop(SfxDest::Channel(chan), Some(PlaybackMode::Loop)));
                // } else {
                self.commands
                    .queue(AudioCommand::Stop(SfxDest::All, Some(PlaybackMode::Loop)));
                // }
            }
            SfxCommand::Play(n) => {
                let sfx = self
                    .pico8_asset()?
                    .audio_banks
                    .get(bank as usize)
                    .ok_or(Error::NoSuch(format!("audio bank {bank}").into()))?
                    .get(n as usize)
                    .ok_or(Error::NoAsset(format!("music {n}").into()))?
                    .clone();

                if let Some(mask) = channel_mask {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::ChannelMask(mask),
                        PlaybackSettings::LOOP,
                    ));
                } else {
                    self.commands.queue(AudioCommand::Play(
                        sfx,
                        SfxDest::Any,
                        PlaybackSettings::LOOP,
                    ));
                }
            }
        }
        Ok(())
    }
}
