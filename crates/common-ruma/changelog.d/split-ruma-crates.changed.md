The vendored `ruma` is a facade again rather than one merged crate: `common`,
`events`, `client-api`, `federation-api`, `appservice-api`, `html`,
`signatures` and `state-res` are separate crates, one per upstream crate, and
`ruma` re-exports them under the module names upstream uses. Code that reaches
these types through `common_ruma::` is unaffected.
