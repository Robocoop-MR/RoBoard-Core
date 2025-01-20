fb-out-location := "src/messages/generated"
fb-src-location := "src/messages/flatbuffers"
flatc-flags := "--rust --filename-suffix '' --rust-module-root-file"

default: gen-fb

_check-flatc:
	@flatc --version

# Generate rust files from the flatbuffers definitions
gen-fb: _check-flatc
	mkdir -p {{fb-out-location}}
	rm -f {{fb-out-location}}/*.rs
	cd {{fb-out-location}} \
		&& flatc {{invocation_dir() + "/" + fb-src-location}}/*.fbs {{flatc-flags}}
	@echo "generated rust files from {{fb-src-location}}/"
