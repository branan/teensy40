TARGET=thumbv7em-none-eabihf
TEENSY_TOOLS=$(ARDUINO)/hardware/tools
TEENSY_POST_COMPILE=$(TEENSY_TOOLS)/teensy_post_compile
OBJCOPY=$(TEENSY_TOOLS)/arm/arm-none-eabi/bin/objcopy
OUTDIR=target/$(TARGET)/release/examples

.PHONY: phony_explicit Makefile clippy format
phony_explicit:

$(OUTDIR)/%.elf: phony_explicit
	cargo +nightly build --example $* --release
	cp $(basename $@) $@

$(OUTDIR)/%.hex: $(OUTDIR)/%.elf
	$(OBJCOPY) -O ihex $< $@

%: $(OUTDIR)/%.hex
	$(TEENSY_POST_COMPILE) -file=$* -path=$(shell pwd)/$(OUTDIR) -tools=$(TEENSY_TOOLS)

clippy:
	cargo +nightly clippy --examples -- -W clippy::all

format:
	cargo +nightly fmt

.PRECIOUS: $(OUTDIR)/%.hex
