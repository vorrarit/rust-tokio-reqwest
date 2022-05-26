## musl build
docker run -it --rm -v "$(pwd):/project" -w="/project" ekidd/rust-musl-builder cargo build --release

## run
./rust-tokio-reqwest 4 input.csv

4 is number of threads
input.csv is input file