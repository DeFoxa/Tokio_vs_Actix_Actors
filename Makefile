.Phony: main concurrency_models
main:
	cargo run --bin ob_stream_concurrency_testing

diesel:
	cargo run --bin diesel_setup

tokio: 
	cargo run --bin tokio_framework

tokio log:
	RUST_LOG=info cargo run --bin tokio_framework

actix: 
	cargo run --bin actix_framework

actix log:
	RUST_LOG=info cargo run --bin actix_framework

test:
	cargo run --bin model_testing


