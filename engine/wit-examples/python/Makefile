SHELL := /bin/bash
ACTIVATE = p_env/bin/activate

init-env:
	python3 -m venv p_env

install-wasmtime-py:
	source ${ACTIVATE} && pip install wasmtime
	source ${ACTIVATE} && pip install pycryptodome
	source ${ACTIVATE} && pip install --upgrade pip

bindings:
	@source ${ACTIVATE} && python3 -m wasmtime.bindgen \
	../../../target/seedwing-policy-engine-component.wasm --out-dir dist

run: 
	@source ${ACTIVATE} && python3 engine.py

pip-list:
	@source ${ACTIVATE} && pip list


clean:
	@${RM} -rf dist
