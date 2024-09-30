BASE_DIR ?= ~/check
BATCH_DIR ?= $(BASE_DIR)/batch
CONFIG_DIR ?= $(BASE_DIR)/config
CONFIGS ?= repos-default.json repos-ui.json
ARGS_CONFIGS ?= $(shell echo "$(CONFIGS)" | awk '{for(i=1;i<=NF;i++) printf("--config %s ", $$i)}')

BATCH_CONFIGS := $(wildcard $(CONFIG_DIR)/*.json)

ifeq ($(PUSH),true)
	# push to database with 
  SINGLE_JSON = $(BATCH_DIR)/single.json
else
  SINGLE_JSON = json
endif

# arg1: config json path
# arg2: output json path
define run_each
	echo "正在处理 $(1)";
	jq ". | to_entries | map(.key)" "$(1)";
	echo "正在设置工具链和检查环境 $(1)";
	os-checker run --config $(1) --emit $(2) --db cache.redb
	echo "完成 $(2)";
	make upload

endef

define make_batch
	os-checker batch $(ARGS_CONFIGS) --out-dir $(CONFIG_DIR) --size 1;

endef

echo:
	echo "$(BASE_DIR)"

upload:
	ls -alh
	gh release upload --clobber -R os-checker/database cache.redb cache.redb
	# XZ_OPT=-e9 tar -cJvf cache.redb.tar.xz cache.redb
	# ls -alh
	# gh release upload --clobber -R os-checker/database cache.redb cache.redb.tar.xz

batch_run:
	$(call make_batch)
	$(foreach config,$(BATCH_CONFIGS),$(call run_each,$(config),$(BATCH_DIR)/$(shell basename $(config))))

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
