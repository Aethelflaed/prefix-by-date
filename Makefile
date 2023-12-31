export PREFIX_BY_DATE_LOG=error,prefix_by_date=debug

install_debug: files/debug.desktop files/patterns.toml
	mkdir -p ~/.local/share/kio/servicemenus
	cp files/debug.desktop ~/.local/share/kio/servicemenus/prefix-by-date.desktop
	update-desktop-database ~/.local/share/kio/servicemenus
	mkdir -p ~/.config/prefix-by-date
	cp files/patterns.toml ~/.config/prefix-by-date/

clean_coverage:
	rm -rf target/coverage

test_coverage: clean_coverage
	CARGO_INCREMENTAL=0 \
										RUSTFLAGS='-Cinstrument-coverage' \
										LLVM_PROFILE_FILE='target/coverage/cargo-test-%p-%m.profraw' \
										cargo test

coverage: test_coverage
	grcov target/coverage --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html

test: install_debug
	make -C tests

text: test
	cargo build -F gui -F text
	./target/debug/prefix-by-date -vvv -i text \
		tests/2023-10-15\ Hello.pdf \
		tests/Hello\ au\ 2023-10-15.pdf \
		tests/Hello\ 2023-10-15.pdf \
		tests/IMG-20231117-whatever.jpg \
		tests/IMG-20231117-another.jpg

gui: test
	cargo build -F gui -F text
	./target/debug/prefix-by-date -vvv -i gui \
		tests/2023-10-15\ Hello.pdf \
		tests/Hello\ au\ 2023-10-15.pdf \
		tests/Hello\ 2023-10-15.pdf \
		tests/IMG-20231117-whatever.jpg \
		tests/IMG-20231117-another.jpg

journal:
	journalctl -e --user -t prefix-by-date
