# Create the core webassembly module
compile:
	@cd static-evaluator && cargo b --target wasm32-wasi --release

component:
	wasm-tools component new static-evaluator/target/wasm32-wasi/debug/static_evaluator.wasm \
	--adapt wasi_snapshot_preview1=../wit-lib/wasi_preview1_component_adapter.command.wasm \
	-o modules/static-evaluator-component.wasm
	@wasm-tools strip modules/static-evaluator-component.wasm -o modules/static-evaluator-component.wasm

copy-engine-component:
	@cp ../../target/seedwing-policy-engine-component.wasm modules/

compose-dog:
	@cd wasm-generator && cargo r -- --policy ../policy-dog.dog \
	       	--name dog \
		--modules ../modules
	@wasm-tools strip wasm-generator/working/target/dog-composed.wasm -o wasm-generator/working/target/dog-composed.wasm

compose-cat:
	@cd wasm-generator && cargo r -- --policy ../policy-cat.dog \
		--name cat \
		--modules ../modules
	@wasm-tools strip wasm-generator/working/target/cat-composed.wasm -o wasm-generator/working/target/cat-composed.wasm

all: compile copy-engine-component component compose run

build-runner: 
	@cd wasm-runtime && cargo b --release

run-dog:
	@cd wasm-runtime && cargo r --release -- \
		--module ../wasm-generator/working/target/dog-composed.wasm \
		--input '{"name": "goodboy", "trained": true}'

run-cat:
	@cd wasm-runtime && cargo r --release -- \
		--module ../wasm-generator/working/target/cat-composed.wasm \
		--input '{"name": "fluffy", "trained": false}'

wit-bindgen:
	wit-bindgen rust ../wit -w static-evaluator --exports world=static-evaluator

show-component-sizes:
	@ls -lh modules/*.wasm wasm-generator/working/target/*composed.wasm | awk '{print $$5, $$9}'
