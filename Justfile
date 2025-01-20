
fb-src-location := "src/flatbuffers"
fb-out-location := fb-src-location + "/generated"

default: gen-fb

_check-flatc:
    @flatc --version

# Generate rust files from the flatbuffers definitions
gen-fb: _check-flatc
    mkdir -p {{fb-out-location}}
    rm -f {{fb-out-location}}/*.rs
    cd {{fb-out-location}} \
        && flatc ../*.fbs --rust --filename-suffix "" --rust-module-root-file
    @echo "generated rust files from {{fb-src-location}}/"
