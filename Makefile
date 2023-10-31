install_debug: files/debug.desktop files/patterns.toml
	mkdir -p ~/.local/share/kio/servicemenus
	cp files/debug.desktop ~/.local/share/kio/servicemenus/prefix-by-date.desktop
	update-desktop-database ~/.local/share/kio/servicemenus
	mkdir -p ~/.config/prefix-by-date
	cp files/patterns.toml ~/.config/prefix-by-date/

