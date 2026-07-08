

define cargo_build

target/$(1)/release/cache_cat $(2):
	cargo zigbuild --release --target $(1)
	mkdir -p dist/
	cp -v target/$(1)/release/cache_cat dist/cache_cat-$(2)
	cp -v target/$(1)/release/benchmark dist/benchmark-$(2)
	cp -v target/$(1)/release/cache_cat_ping dist/cache_cat_ping-$(2)

TOOLCHAIN+= $(1)
ALL_TARGET+= $(2)

endef

$(eval $(call cargo_build,x86_64-unknown-linux-musl,linux-amd64))
$(eval $(call cargo_build,aarch64-unknown-linux-musl,linux-arm64-v8))
$(eval $(call cargo_build,riscv64gc-unknown-linux-musl,linux-riscv64))
$(eval $(call cargo_build,x86_64-pc-windows-gnu,windows-amd64))
# $(eval $(call cargo_build,armv7-unknown-linux-musleabihf,linux-arm))
# $(eval $(call cargo_build,s390x-unknown-linux-musl,linux-s390x))
# $(eval $(call cargo_build,loongarch64-unknown-linux-musl,linux-loongarch64))


install_toolchain:
	rustup target add $(TOOLCHAIN)

fetch:
	cargo fetch

all: fetch $(ALL_TARGET)


.PHONY: fetch $(ALL_TARGET)