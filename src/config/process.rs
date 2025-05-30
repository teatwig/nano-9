use bevy::{
    utils::ConditionalSendFuture,
    asset::{io::{Reader, AsyncWriteExt}, AssetLoader, AssetPath, LoadContext,
            meta::AssetMeta,
            processor::{ProcessContext, Process, ProcessError}},
    prelude::*,
};

// // This won't be able to handle includes.
// pub struct LuaProcess;

// impl Process for LuaProcess {
//     type Settings = ();
//     type OutputLoader = ScriptAssetLoader;

//     async fn process(
//         &self,
//         context: &mut ProcessContext<'_>,
//         meta: AssetMeta<(), Self>,
//         writer: &mut bevy::asset::io::Writer,
//     ) -> Result<<Self::OutputLoader as bevy::asset::AssetLoader>::Settings, ProcessError> {

//         let mut code = std::str::from_utf8(context.asset_bytes()).unwrap();
//         // dbg!(1);
//         // #[cfg(feature = "pico8-to-lua")]
//         // {
//         //     if context.path().path().extension().map(|x| x == "p8lua").unwrap_or(false) {
//         // dbg!(2);
//         //         if let Some(patched_code) = pico8::translate_pico8_to_lua(&code, load_context).await? {
//         // dbg!(3);
//         //             code = patched_code;
//         //         }
//         //     }
//         // }

//         warn!("WHAT");
//         dbg!("processing");
//         writer.write_all(code.as_bytes()).await.unwrap();
//         Ok(())
//     }
// }
