# Makefile for lx-zed extension

# Target architecture (Rust 1.78+ uses wasm32-wasip1)
TARGET := wasm32-wasip1

# The name of the output extension file
EXTENSION_ID := lx
OUTPUT_WASM := extension.wasm

# The actual name of the compiled binary (crate name with _ instead of -)
CRATE_NAME := lx_zed

all: build

setup:
	@echo "Installing Rust target $(TARGET)..."
	rustup target add $(TARGET)

build: setup
	@echo "Building extension..."
	cargo build --release --target $(TARGET)
	@echo "Copying binary to $(OUTPUT_WASM)..."
	# FIX: Explicitly copy the crate binary to avoid wildcard errors
	cp target/$(TARGET)/release/$(CRATE_NAME).wasm $(OUTPUT_WASM)
	@echo "Build complete: $(OUTPUT_WASM)"

clean:
	@echo "Cleaning..."
	cargo clean
	rm -f $(OUTPUT_WASM)

install: build
	@echo "Installing to Zed extensions directory..."
	mkdir -p ~/Library/Application\ Support/Zed/extensions/installed/$(EXTENSION_ID)
	cp $(OUTPUT_WASM) ~/Library/Application\ Support/Zed/extensions/installed/$(EXTENSION_ID)/
	cp extension.toml ~/Library/Application\ Support/Zed/extensions/installed/$(EXTENSION_ID)/
	@echo "Installed! Restart Zed to reload."

.PHONY: all setup build clean install
