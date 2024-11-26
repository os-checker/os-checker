BASE_DIR ?= ~/check
BATCH_DIR ?= $(BASE_DIR)/batch
# CONFIG_DIR ?= $(BASE_DIR)/config
OS_CHECKER_CONFIGS ?= repos-default.json repos-ui.json
TAG_PRECOMPILED_CHECKERS ?= precompiled-checkers

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
	gh release upload --clobber -R os-checker/database $(TAG_PRECOMPILED_CHECKERS)  ~/.cargo/bin/os-checker
	XZ_OPT=-e9 tar -cJvf os-checker.tar.xz -C ~/.cargo/bin/ os-checker os-checker-database batch
	gh release upload --clobber -R os-checker/database $(TAG_PRECOMPILED_CHECKERS)  os-checker.tar.xz

run:
	@OS_CHECKER_CONFIGS="$(OS_CHECKER_CONFIGS)" os-checker run --emit $(SINGLE_JSON) --db cache.redb

# author zjp-CN, and commiter bot
clone_database:
	@git config --global user.name "zjp-CN[bot]"
	@git config --global user.email "zjp-CN[bot]@users.noreply.github.com"
	@git config --global committer.name "zjp-CN[bot]"
	@git config --global committer.email "zjp-CN[bot]@users.noreply.github.com"
	@
	@echo "正在 clone os-checker/database"
	@git clone https://x-access-token:$(ACCESS_TOKEN)@github.com/os-checker/database.git
	@echo "成功 clone os-checker/database"

# print repos info without installing anything
layout:
	@OS_CHECKER_CONFIGS="$(OS_CHECKER_CONFIGS)" os-checker layout 2>&1 | tee $(BATCH_DIR)/layout.txt

layout_list_targets:
	cd $(BASE_DIR) && OS_CHECKER_CONFIGS="$(OS_CHECKER_CONFIGS)" os-checker layout --list-targets seL4/rust-sel4
