Writes that persist Olm session state now use strict IndexedDB durability. The
default durability lets the browser acknowledge a write that is still only in
the operating system's buffers, so a crash or power cut could leave the stored
session behind the one the other side had already seen, wedging it and producing
UTDs.
