set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

id_full := `cargo pkgid --offline`
id := replace_regex(id_full, '^.*#', '')
win_name := "ttdl_"+id+"_win_x64.zip"
lin_name := "ttdl-"+id+"-linux-x64-musl.tar.gz"

pkg-win:
	echo "winname: {{win_name}}"
	if (Test-Path -Path "{{win_name}}" -PathType Leaf) { throw "{{win_name}} already exists" }
	@echo "Packaging {{id}} for Windows..."
	7z a "{{win_name}}" -tzip .\target\release\ttdl.exe .\README.md .\changelog .\LICENSE .\ttdl.toml

pkg-musl:
	if [ -s "./target/pkg/{{lin_name}}" ]; then echo "./target/pkg/{{lin_name}} already exists" && exit 1; fi
	@echo "Packaging {{id}} for Linux(musl)..."
	rm -rf ./target/pkg
	mkdir ./target/pkg
	cp ./README.md ./target/pkg/
	cp ./changelog ./target/pkg/
	cp ./LICENSE ./target/pkg/
	cp ./ttdl.toml ./target/pkg/
	cp ./target/x86_64-unknown-linux-musl/release/ttdl ./target/pkg/
	cd ./target/pkg && tar -czvf "{{lin_name}}" ttdl README.md changelog ttdl.toml LICENSE
