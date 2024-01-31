SIGNER=
KEYCHAIN_PROFILE=
# x86_64-apple-darwin or aarch64-apple-darwin
ARCH=
DMG_NAME="Mira Sharer $(ARCH).dmg"
BUILD_DIR=./build/$(ARCH)

FFMPEG_ENV_DIR=

default:
	@echo ":)"

bundle-macos:
	if [ -z "$(SIGNER)" ]; then echo "SIGNER not set"; exit 1; fi
	if [ -z "$(KEYCHAIN_PROFILE)" ]; then echo "KEYCHAIN_PROFILE not set"; exit 1; fi
	if [ -z "$(FFMPEG_ENV_DIR)" ]; then echo "FFMPEG_ENV_DIR not set"; exit 1; fi
	if [ "$(ARCH)" = "x86_64-apple-darwin" ]; then \
  		echo "Building for X86_64"; \
	elif [ "$(ARCH)" = "aarch64-apple-darwin" ]; then \
	  	echo "Building for ARM64"; \
	else \
		echo "ARCH not set, must be x86_64-apple-darwin or aarch64-apple-darwin"; \
		exit 1; \
	fi
	MACOSX_DEPLOYMENT_TARGET=13.0 \
	PKG_CONFIG_PATH=$(FFMPEG_ENV_DIR)/lib/pkgconfig \
	FFMPEG_INCLUDE_DIR=$(FFMPEG_ENV_DIR)/include \
	FFMPEG_LIB_DIR=$(FFMPEG_ENV_DIR)/lib \
	cargo bundle --release --target $(ARCH);
	dylibbundler \
		-b \
		-x target/$(ARCH)/release/bundle/osx/Mira\ Sharer.app/Contents/MacOS/mira_sharer \
		-cd \
		-d target/$(ARCH)/release/bundle/osx/Mira\ Sharer.app/Contents/Frameworks/ \
		-p @executable_path/../Frameworks/ \
		-s $(FFMPEG_ENV_DIR)/lib
	codesign \
		--options runtime \
		-vvvvv \
		-f \
		-s "$(SIGNER)" \
		target/$(ARCH)/release/bundle/osx/Mira\ Sharer.app/Contents/Frameworks/* \
		target/$(ARCH)/release/bundle/osx/Mira\ Sharer.app
	mkdir -p $(BUILD_DIR)
	rm -f $(BUILD_DIR)/$(DMG_NAME)
	create-dmg target/$(ARCH)/release/bundle/osx/Mira\ Sharer.app/ $(BUILD_DIR) || true
	mv $(BUILD_DIR)/Mira*.dmg $(BUILD_DIR)/$(DMG_NAME)
	xcrun notarytool submit $(BUILD_DIR)/$(DMG_NAME) --keychain-profile "$(KEYCHAIN_PROFILE)" --wait
	@echo "DMG built at $(BUILD_DIR)/$(DMG_NAME)"
