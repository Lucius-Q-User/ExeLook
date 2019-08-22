all: ExeLook.qlgenerator

install: all
	cp -r ExeLook.qlgenerator ~/Library/QuickLook
	qlmanage -r

target/release/libexe_look.dylib: Cargo.toml $(wildcard src/*.rs src/**/*.rs)
	cargo build --release

ExeLook.qlgenerator: Info.plist target/release/libexe_look.dylib
	mkdir -p ExeLook.qlgenerator/Contents/MacOS
	cp target/release/libexe_look.dylib ExeLook.qlgenerator/Contents/MacOS/
	cp Info.plist ExeLook.qlgenerator/Contents/

clean:
	-rm -rf ExeLook.qlgenerator
	cargo clean

.PHONY: clean all
