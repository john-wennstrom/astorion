#[macro_export]
macro_rules! regex {
    ($pat:literal) => {{
        static RE: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new($pat).unwrap());
        &*RE
    }};
}

#[macro_export]
macro_rules! re {
    ($pat:literal) => {
        $crate::Pattern::Regex($crate::regex!($pat))
    };
}

#[macro_export]
macro_rules! pred {
    ($p:expr) => {
        $crate::Pattern::Predicate($p)
    };
}

#[macro_export]
macro_rules! rule {
    (
        name: $name:expr,
        pattern: [ $($pat:expr),* $(,)? ]
        $(, required_phrases: [ $($req_phrase:expr),* $(,)? ])?
        $(, optional_phrases: [ $($opt_phrase:expr),* $(,)? ])?
        $(, buckets: $buckets:expr)?
        $(, deps: [ $($dep:expr),* $(,)? ])?
        $(, priority: $priority:expr)?
        , prod: |$tokens_expr:ident : &[$tok_ty_expr:ty]| -> $ret_ty:ty $body_expr:block
        $(,)?
    ) => {{
        $crate::Rule {
            name: $name,
            pattern: vec![ $($pat),* ],
            production: Box::new(move |$tokens_expr: &[$tok_ty_expr]| {
                use $crate::IntoToken;
                let result: $ret_ty = $body_expr;
                result.and_then(|v| v.into_token())
            }),
            required_phrases: &[ $($($req_phrase),*)? ],
            optional_phrases: &[ $($($opt_phrase),*)? ],
            buckets: { 0 $(| $buckets)? },
            deps: &[ $($($dep),*)? ],
            priority: { 0 $(+ $priority)? },
        }
    }};
}
