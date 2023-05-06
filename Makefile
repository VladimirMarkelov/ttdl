IDFULL=$(shell cargo pkgid --offline)
ID=$(lastword $(subst \#, ,$(IDFULL)))
WINNAME=ttdl_$(ID)_win_x64.zip
LINNAME=ttdl_$(ID)_linux-x64-musl.tar.gz

.PHONY: pkg-win pkg-musl

pkg-win:
	@echo Creating windows package: $(WINNAME)...
ifneq ("$(wildcard $(WINNAME))", "")
	$(error "$(WINNAME) already exists")
endif
	7z a "$(WINNAME)" -tzip .\target\release\ttdl.exe .\README.md .\changelog .\LICENSE .\ttdl.toml

pkg-musl:
	@echo Creating linux musl package: $(LINNAME)...
ifneq ("$(wildcard $(LINNAME))", "")
	$(error "$(LINNAME) already exists")
endif
	rm -rf ./target/pkg
	mkdir ./target/pkg
	cp ./README.md ./target/pkg/
	cp ./changelog ./target/pkg/
	cp ./LICENSE ./target/pkg/
	cp ./ttdl.toml ./target/pkg/
	cp ./target/x86_64-unknown-linux-musl/release/ttdl ./target/pkg/
	cd ./target/pkg && tar -czvf "$(LINNAME)" ttdl README.md changelog ttdl.toml LICENSE
