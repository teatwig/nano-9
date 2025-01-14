macro_rules! define_globals {
    (
        $(fn $name:ident($ctx:ident, $arg_name:tt : $arg_type:tt) $body:block)+
    ) => {
        $(
            #[allow(unused_parens)]
            #[allow(unused_variables)]
            $ctx.globals()
                .set(stringify!($name),
                     $ctx.create_function(|$ctx, $arg_name: $arg_type| $body)
                     .map_err(ScriptError::new_other)?
                ).map_err(ScriptError::new_other)?;
        )+
    };
}
pub(crate) use define_globals;
