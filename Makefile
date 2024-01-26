.Phony: main concurrency_models
main:
	cargo run --bin ob_stream_concurrency_testing

diesel-setup:
	cargo run --bin diesel_setup

tokio: 
	cargo run --bin tokio_framework

actix: 
	cargo run --bin actix_framework

