

define cargo_build

target/$(1)/release/cache_cat:
	cargo zigbuild --release --target $(1)

dist/cache_cat-$(1): target/$(1)/release/cache_cat
	mkdir -p dist/
	cp -v target/$(1)/release/cache_cat dist/cache_cat-$(1)
	cp -v target/$(1)/release/cache_cat_ping dist/cache_cat_ping-$(1)

TOOLCHAIN+= $1
ALL_TARGET+= dist/cache_cat-$(1)
.PHONY: target/$(1)/release/cache_cat
endef

install_toolchain:
	rustup target add $(TOOLCHAIN)

$(eval $(call cargo_build,x86_64-unknown-linux-musl,linux/amd64))
$(eval $(call cargo_build,aarch64-unknown-linux-musl,linux/arm64/v8))


fetch:
	cargo fetch

all: fetch $(ALL_TARGET)


.PHONY: fetch