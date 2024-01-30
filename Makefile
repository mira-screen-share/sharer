SIGNER=
KEYCHAIN_PROFILE=

default:
	@echo "."

bundle-macos:
	if [ -z "$(SIGNER)" ]; then echo "SIGNER not set"; exit 1; fi
	if [ -z "$(KEYCHAIN_PROFILE)" ]; then echo "KEYCHAIN_PROFILE not set"; exit 1; fi
	cargo bundle --release
	dylibbundler \
		-b \
		-x target/release/bundle/osx/Mira\ Sharer.app/Contents/MacOS/mira_sharer \
		-cd \
		-d target/release/bundle/osx/Mira\ Sharer.app/Contents/Frameworks/ \
		-p @executable_path/../Frameworks/
	codesign \
		--options runtime \
		-vvvvv \
		-f \
		-s "$(SIGNER)" \
		/Users/mark/repo/mira/sharer/target/release/bundle/osx/Mira\ Sharer.app/Contents/Frameworks/* \
		/Users/mark/repo/mira/sharer/target/release/bundle/osx/Mira\ Sharer.app
	rm -f Mira\ Sharer.dmg
	create-dmg target/release/bundle/osx/Mira\ Sharer.app/ || true
	mv Mira*.dmg Mira\ Sharer.dmg
	xcrun notarytool submit Mira\ Sharer.dmg --keychain-profile "$(KEYCHAIN_PROFILE)" --wait
