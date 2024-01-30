default:
	@echo "."

bundle-macos:
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
		-s "Developer ID Application: Anhai Wang (MPPUFL3L6R)" \
		/Users/mark/repo/mira/sharer/target/release/bundle/osx/Mira\ Sharer.app/Contents/Frameworks/* \
		/Users/mark/repo/mira/sharer/target/release/bundle/osx/Mira\ Sharer.app
	rm -f Mira\ Sharer.dmg
	create-dmg target/release/bundle/osx/Mira\ Sharer.app/ || true
	mv Mira*.dmg Mira\ Sharer.dmg
	xcrun notarytool submit Mira\ Sharer.dmg --keychain-profile "default" --wait
