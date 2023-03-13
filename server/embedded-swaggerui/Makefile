.PHONY: all
all: update

.PHONY: clean
clean:
	rm -Rf download
	rm -Rf unpack

.PHONY: update
update: clean
	rm -Rf dist
	mkdir -p download
	curl -sSL https://github.com/swagger-api/swagger-ui/archive/refs/tags/v4.18.0.tar.gz -o download/swaggerui.tar.gz
	mkdir -p unpack
	cd unpack && tar --strip-components=1 -xzf ../download/swaggerui.tar.gz
	mkdir -p dist
	mv unpack/dist/* dist/
	find dist
