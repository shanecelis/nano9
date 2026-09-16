GOLDEN_DIR := tests/golden
SFX_DIR := $(GOLDEN_DIR)/sfx
IMAGE_CARTS := $(wildcard $(GOLDEN_DIR)/*.p8)
SFX_CARTS := $(wildcard $(SFX_DIR)/*.p8)
MUSIC_CARTS := $(wildcard $(GOLDEN_DIR)/music-*.p8)
EXPECTED := \
	$(patsubst $(GOLDEN_DIR)/%.p8,$(GOLDEN_DIR)/%-expected.png,$(IMAGE_CARTS)) \
	$(patsubst $(SFX_DIR)/%.p8,$(SFX_DIR)/%-expected.wav,$(SFX_CARTS)) \
	$(patsubst $(GOLDEN_DIR)/%.p8,$(GOLDEN_DIR)/%-expected.wav,$(MUSIC_CARTS))

.PHONY: golden golden-synth
golden: $(EXPECTED)

# Interactive: opens Pico-8; type `EXPORT SYNTH%D.WAV` then `SHUTDOWN`.
golden-synth:
	bin/golden-synth

$(GOLDEN_DIR)/%-expected.png: $(GOLDEN_DIR)/%.p8
	bin/golden-pico8 $< -o $@

$(SFX_DIR)/%-expected.wav: $(SFX_DIR)/%.p8
	bin/golden-pico8 $< -o $@

$(GOLDEN_DIR)/%-expected.wav: $(GOLDEN_DIR)/%.p8
	bin/golden-pico8 $< -o $@
