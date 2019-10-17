# Computer-Network-Assignment-1
This is the server for the assignment


The Client is here https://github.com/HoloPollock/network_assignment_1_client
## Run
How to run all these assume a Unix like experience
1. Install Rust  
Use [rustup](https://rustup.rs) to install rust
2. Clone Repo
3. Set Rust toolchain to nightly
Once in Repo this requires the nightly version of Rust to get the nighty version run either `rustup default nightly` to set the default toolchain to nighty or you can use per-directory overrides to use the nightly version by within the directory run `rustup override set nightly`
4. Use cargo to run
To run the server use the command `cargo run` becuse of the way the code assume file do not run it from anywhere other than `/Networks-Assignment-1`
