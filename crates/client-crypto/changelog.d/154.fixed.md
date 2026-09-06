Resetting the cross-signing identity is now serialised against `/keys/query`
processing. A response landing in the middle saw a public identity it didn't
recognise and threw the freshly created private keys away, leaving an account
that could log in but could not set up recovery afterwards.
