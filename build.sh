# compiling on linux
cargo rustc -- -C link-arg=-nostartfiles

# compiling on macOS
#cargo rustc -- -C link-args="-e __start -static -nostartfiles"

# compiling on Windows (change the file extension)
#cargo rustc -- -C link-args="/ENTRY:_start /SUBSYSTEM:console"
