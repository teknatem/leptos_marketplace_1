# PROMPT: Generate a Python script using the 'os', 'os.path', and 'json' modules.
# The script must analyze the file structure of a Rust project focused on Domain Aggregates (a001_ to a016_) and collect file sizes (in bytes) to build a structured JSON output.

# 1. Configuration: List of all 16 aggregate prefixes.
AGGREGATE_PREFIXES = [
    "a001_connection_1c", "a002_organization", "a003_counterparty", "a004_nomenclature",
    "a005_marketplace", "a006_connection_mp", "a007_marketplace_product", "a008_marketplace_sales",
    "a009_ozon_returns", "a010_ozon_fbs_posting", "a011_ozon_fbo_posting", "a012_wb_sales",
    "a013_ym_order", "a014_ozon_transactions", "a015_wb_orders", "a016_ym_returns"
]

# 2. Define the exact file paths expected for the fixed columns.
FILE_PATHS = {
    "contracts": {"aggregate.rs": "{root}/crates/contracts/src/domain/{agg}/aggregate.rs"},
    "backend": {
        "repository.rs": "{root}/crates/backend/src/domain/{agg}/repository.rs",
        "service.rs": "{root}/crates/backend/src/domain/{agg}/service.rs",
        "posting.rs": "{root}/crates/backend/src/domain/{agg}/posting.rs", # May not exist (size 0)
    },
    "handlers_base": "{root}/crates/backend/src/handlers/"
}

# 3. Implement the core logic in Python.

import os
import json
import glob

def get_file_size(filepath):
    """Safely return file size in bytes, or 0 if file does not exist."""
    try:
        return os.path.getsize(filepath)
    except FileNotFoundError:
        return 0

def find_first_file(directory, patterns):
    """Find the first existing file matching any pattern and return its size and path."""
    for pattern in patterns:
        search_path = os.path.join(directory, pattern)
        for filepath in glob.glob(search_path):
            if os.path.exists(filepath):
                return get_file_size(filepath), filepath
    return 0, None

def get_directory_size(directory):
    """Recursively sum size of all .rs files in directory."""
    total_size = 0
    if not os.path.exists(directory):
        return 0
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith('.rs'):
                filepath = os.path.join(root, file)
                total_size += get_file_size(filepath)
    return total_size

def collect_metrics(root_dir):
    metrics = {}

    for agg in AGGREGATE_PREFIXES:
        # --- Paths based on aggregate prefix ---
        contracts_dir = os.path.join(root_dir, 'crates', 'contracts', 'src', 'domain', agg)
        backend_dir = os.path.join(root_dir, 'crates', 'backend', 'src', 'domain', agg)
        frontend_ui_dir = os.path.join(root_dir, 'crates', 'frontend', 'src', 'domain', agg, 'ui')
        handlers_search_dir = os.path.join(root_dir, 'crates', 'backend', 'src', 'handlers')

        # Dictionary to track all file paths explicitly processed (to calculate Misc)
        processed_files = set()
        
        agg_metrics = {
            "contracts": {"aggregate.rs": 0, "misc": 0},
            "backend": {"repository.rs": 0, "service.rs": 0, "posting.rs": 0, "misc": 0},
            "frontend": {"details": 0, "list": 0, "picker": 0, "tree": 0, "misc": 0},
            "handlers": {"handler_file": 0}
        }

        # --- 1. Contracts ---
        contracts_agg_path = FILE_PATHS["contracts"]["aggregate.rs"].format(root=root_dir, agg=agg)
        agg_metrics["contracts"]["aggregate.rs"] = get_file_size(contracts_agg_path)
        processed_files.add(contracts_agg_path)
        
        # --- 2. Backend ---
        for filename, path_template in FILE_PATHS["backend"].items():
            filepath = path_template.format(root=root_dir, agg=agg)
            size = get_file_size(filepath)
            agg_metrics["backend"][filename] = size
            processed_files.add(filepath)
        
        # --- 3. Frontend ---
        
        # Frontend: Details - все файлы в ui/details/
        details_dir = os.path.join(frontend_ui_dir, 'details')
        agg_metrics["frontend"]["details"] = get_directory_size(details_dir)
        
        # Frontend: List - все файлы в ui/list/
        list_dir = os.path.join(frontend_ui_dir, 'list')
        agg_metrics["frontend"]["list"] = get_directory_size(list_dir)
        
        # Frontend: Picker - все файлы в ui/picker/
        picker_dir = os.path.join(frontend_ui_dir, 'picker')
        agg_metrics["frontend"]["picker"] = get_directory_size(picker_dir)
        
        # Frontend: Tree - все файлы в ui/tree/
        tree_dir = os.path.join(frontend_ui_dir, 'tree')
        agg_metrics["frontend"]["tree"] = get_directory_size(tree_dir)
        
        # Отслеживаем все обработанные директории для misc
        processed_directories = {details_dir, list_dir, picker_dir, tree_dir}


        # --- 4. Handlers (Single File, Outside Domain) ---
        # Check if handler file exists for this aggregate
        handler_path = os.path.join(handlers_search_dir, f'{agg}.rs')
        if os.path.exists(handler_path):
            agg_metrics["handlers"]["handler_file"] = get_file_size(handler_path)
            processed_files.add(handler_path)
            # If the handler is A001, its handlers are often bundled in main.rs, which is 60k bytes.
            # However, for consistency and granular monitoring, we only track the specific handler file if it exists.

        # --- 5. Calculate Misc Files ---
        
        # 5.1 Contracts Misc
        contracts_misc_size = 0
        for root, _, files in os.walk(contracts_dir):
            for file in files:
                if file.endswith('.rs'):
                    filepath = os.path.join(root, file)
                    if filepath not in processed_files:
                        contracts_misc_size += get_file_size(filepath)
        agg_metrics["contracts"]["misc"] = contracts_misc_size
        
        # 5.2 Backend Misc (Excluding UseCases/Imports explicit in the prompt)
        backend_misc_size = 0
        # Files to exclude from misc, besides the standard ones: u501_import*, excel_import*
        EXCLUSION_PATTERNS = ('u501_import', 'u502_import', 'u503_import', 'u504_import', 'u505_match', 'u506_import', 'excel_import')
        for root, _, files in os.walk(backend_dir):
            for file in files:
                if file.endswith('.rs'):
                    filepath = os.path.join(root, file)
                    
                    # 5.2.1 Check against explicit processed set
                    if filepath in processed_files:
                        continue
                        
                    # 5.2.2 Check against exclusion patterns (UseCases)
                    is_usecase = False
                    for pattern in EXCLUSION_PATTERNS:
                        if file.startswith(pattern):
                            is_usecase = True
                            break
                            
                    if not is_usecase:
                        backend_misc_size += get_file_size(filepath)

        agg_metrics["backend"]["misc"] = backend_misc_size

        # 5.3 Frontend Misc - исключаем файлы из details, list, picker, tree
        frontend_misc_size = 0
        for root, _, files in os.walk(frontend_ui_dir):
            # Проверяем, не находимся ли мы в одной из обработанных директорий
            is_in_processed = any(root.startswith(proc_dir) for proc_dir in processed_directories if os.path.exists(proc_dir))
            if not is_in_processed:
                for file in files:
                    if file.endswith('.rs'):
                        filepath = os.path.join(root, file)
                        frontend_misc_size += get_file_size(filepath)
        agg_metrics["frontend"]["misc"] = frontend_misc_size


        metrics[agg] = agg_metrics

    return metrics

# --- Execution Block ---
# Assuming the script runs from the project root where 'crates' is located.
ROOT_DIR = os.getcwd()
OUTPUT_FILE = 'aggregate_metrics.json'

# Execute the collection
collected_metrics = collect_metrics(ROOT_DIR)

# Write to JSON file
with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
    json.dump(collected_metrics, f, indent=2, ensure_ascii=False)

print(f"Metrics collected successfully and saved to {OUTPUT_FILE}")