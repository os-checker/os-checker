BASE_DIR ?= ~/check
BATCH_DIR ?= $(BASE_DIR)/batch
OUTPUI_DIR ?= $(BASE_DIR)/output
CONFIGS ?= repos.json repos-ui.json
ARGS_CONFIGS ?= $(shell echo "$(CONFIGS)" | awk '{for(i=1;i<=NF;i++) printf("--config %s ", $$i)}')

BATCH_CONFIGS := $(wildcard $(BATCH_DIR)/*.json)

# arg1: config json path
# arg2: output json path
define run_each
	echo "正在处理 $(1)";
	jq ". | to_entries | map(.key)" "$(1)";
	echo "正在设置工具链和检查环境 $(1)";
	os-checker setup --config $(1) --emit json
	echo "设置工具链和检查环境成功 $(1)";
	os-checker run --config $(1) --emit $(2);
	echo "完成 $(2)";

endef

define make_batch
	os-checker batch $(ARGS_CONFIGS) --out-dir $(BATCH_DIR) --size 4;

endef

echo:
	echo "$(BASE_DIR)"

batch:
	@$(call make_batch)

run:
	$(foreach config,$(BATCH_CONFIGS),$(call run_each,$(config),$(OUTPUI_DIR)/$(shell basename $(config))))
