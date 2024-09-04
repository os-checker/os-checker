BASE_DIR ?= ~/check
BATCH_DIR ?= $(BASE_DIR)/batch
OUTPUI_DIR ?= $(BASE_DIR)/output

BATCH_CONFIGS := $(wildcard $(BATCH_DIR)/*.json)

# arg1: config json path
# arg2: output json path
define run_each
	echo "正在处理 $(1)";
	jq ". | to_entries | map(.key)" "$(1)";
	echo "正在设置工具链和检查环境 $(1)";
	# os-checker setup --config $(1) --emit json
	echo "设置工具链和检查环境成功 $(1)";
	# os-checker run --config $(1) --emit $(2);
	echo "完成 $(2)";

endef

echo:
	echo "$(BASE_DIR)"

batch:
	os-checker batch --config assets/repos-ui.json --out-dir $(BATCH_DIR) --size 10

run:
	@mkdir -p $(OUTPUI_DIR)
	@$(foreach config,$(BATCH_CONFIGS),$(call run_each,$(config),$(OUTPUI_DIR)/$(shell basename $(config))))
