
ifeq ($(Build), debug)
	BUILD_TYPE = debug
else
	BUILD_TYPE = release
	BUILD = "--$(BUILD_TYPE)"
endif

corewasm=../target/wasm32-wasi/${BUILD_TYPE}/seedwing_policy_engine.wasm
component=../target/seedwing-policy-engine-component.wasm


# Seedwing WebAssembly Component targets
wit-compile:
	cargo b ${BUILD} -p seedwing-policy-engine --target=wasm32-wasi --no-default-features --features=""

# Can be used to inspect the custom section "component-type::engine-world"
objdump-core-module:
	@wasm-tools objdump $(corewasm)

test_from_engine_runtime:
	cargo t -p seedwing-policy-engine --target=wasm32-wasi \
	--no-default-features --features="" \
	--lib core::wit::test::from_engine_runtime -- --exact --show-output

wit-component:
	wasm-tools component new \
	-v $(corewasm) \
	--adapt wasi_snapshot_preview1=wit-lib/wasi_preview1_component_adapter.wasm \
	-o $(component)
	@wasm-tools strip $(component) -o $(component)

wit-print-wat:
	wasm-tools print $(component)

wit-print:
	@wasm-tools component wit $(component)

objdump-component:
	@wasm-tools objdump $(component)

# Can be used to generate the Rust source for that the bindgen::generate
# macro will produce.
wit-bindgen:
	wit-bindgen rust wit/ -w engine-world

# Can be useful to inspect the expanded wit-bindgen macros
cargo-expand:
	cargo expand -p seedwing-policy-engine --target=wasm32-wasi --no-default-features --features=""

# JavaScript targets
wit-javascript-bindings:
	cd wit-examples/javascript && npm run bindings

wit-javascript-run:
	cd wit-examples/javascript && npm start

wit-javascript-all: wit-compile wit-component wit-js-build

# Python targets
wit-python-bindings:
	@cd wit-examples/python && make --no-print-directory bindings

wit-python-run:
	@cd wit-examples/python && make --no-print-directory run

wit-python-all: wit-compile wit-component wit-python-bindings wit-python-run

# Rust targets
wit-rust-bindings:
	cd wit-examples/rust && cargo b ${BUILD}

wit-rust-run:
	cd wit-examples/rust && cargo r ${BUILD}

wit-rust-all: wit-rust-run
