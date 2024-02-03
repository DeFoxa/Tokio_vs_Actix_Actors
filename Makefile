.Phony: main concurrency_models
main:
	cargo run --bin ob_stream_concurrency_testing

diesel:
	cargo run --bin diesel_setup

tokio: 
	cargo run --bin tokio_framework

actix: 
	cargo run --bin actix_framework

test:
	cargo run --bin model_testing

tui:
	cargo run --bin testing_tui 

debug kompact:
	cargo run --bin kompact_framework

kompact:
	cargo run --release --bin kompact_framework
