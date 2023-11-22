SHELL := bash

# Define the test target
.PHONY: test
test:
	cargo test -- --test-threads=1

# This is a phony target which means it doesn't represent a file.