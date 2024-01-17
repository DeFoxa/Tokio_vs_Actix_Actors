.Phony: main diesel_setup
main:
	cargo run --bin ob_stream_concurrency_testing

diesel-setup:
	cargo run --bin diesel_setup

