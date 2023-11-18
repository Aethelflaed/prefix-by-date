install_debug: files/debug.desktop files/patterns.toml
	mkdir -p ~/.local/share/kio/servicemenus
	cp files/debug.desktop ~/.local/share/kio/servicemenus/prefix-by-date.desktop
	update-desktop-database ~/.local/share/kio/servicemenus
	mkdir -p ~/.config/prefix-by-date
	cp files/patterns.toml ~/.config/prefix-by-date/

test:
	make -C tests

try:
	./target/debug/prefix-by-date -i text -vv tests/2023-10-15\ Hello\ .pdf tests/Hello\ au\ 2023-10-15.pdf tests/IMG-20231117-whatever.jpg tests/IMG-20231117-another.jpg
