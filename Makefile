BASE_DIR ?= ~/check
BATCH_DIR ?= $(BASE_DIR)/batch
# CONFIG_DIR ?= $(BASE_DIR)/config
CONFIGS ?= repos-default.json repos-ui.json repos-embassy.json
ARGS_CONFIGS ?= $(shell echo "$(CONFIGS)" | awk '{for(i=1;i<=NF;i++) printf("--config %s ", $$i)}')

ifeq ($(PUSH),true)
	# push to database with 
  SINGLE_JSON = $(BATCH_DIR)/single.json
else
  SINGLE_JSON = json
endif

upload:
	gh release upload --clobber -R os-checker/database $(TAG_CACHE) cache.redb
	XZ_OPT=-e9 tar -cJvf cache.redb.tar.xz cache.redb
	ls -alh
	gh release upload --clobber -R os-checker/database $(TAG_CACHE) cache.redb.tar.xz
	gh release upload --clobber -R os-checker/database $(TAG_CACHE) ~/.cargo/bin/os-checker

run:
	@os-checker run $(ARGS_CONFIGS) --emit $(SINGLE_JSON) --db cache.redb

# author zjp-CN, and commiter bot
clone_database:
	@git config --global user.name "zjp-CN"
	@git config --global user.email "jiping_zhou@foxmail.com"
	@git config --global committer.name "zjp-CN[bot]"
	@git config --global committer.email "zjp-CN[bot]@users.noreply.github.com"
	@
	@echo "正在 clone os-checker/database"
	@git clone https://x-access-token:$(ACCESS_TOKEN)@github.com/os-checker/database.git
	@echo "成功 clone os-checker/database"

# print repos info without installing anything
layout:
	@os-checker layout $(ARGS_CONFIGS) 2>&1 | tee $(BATCH_DIR)/layout.txt

audit:
	gh release download --clobber -R os-checker/database $(TAG_CACHE) -p cargo-audit -D ~/.cargo/bin/ || make install_audit

install_audit:
	ls -alh ~/.cargo/bin/ \
		&& cd ~ \
		&& git clone https://github.com/rustsec/rustsec.git \
		&& cd rustsec \
		&& cargo install --path cargo-audit --force \
		&& gh release upload --clobber -R os-checker/database $(TAG_CACHE) ~/.cargo/bin/cargo-audit
