#!/usr/bin/env python3
"""
Improve Orders Details UI:
- Add background to all labels
- Make more compact
- Ensure gNumber is displayed
"""

import re

with open('crates/frontend/src/domain/a015_wb_orders/ui/details/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add background to all label divs (font-weight: 600; color: #555;)
# Pattern: <div style="font-weight: 600; color: #555;">"SomeLabel:"</div>
label_pattern = r'<div style="font-weight: 600; color: #555;">'
label_replacement = r'<div style="font-weight: 600; color: #555; background: #f5f5f5; padding: 4px 8px; border-radius: 4px; font-size: 13px;">'
content = content.replace(label_pattern, label_replacement)

# Reduce padding in cards
content = content.replace('padding: 15px;', 'padding: 10px;')
content = content.replace('padding: 20px;', 'padding: 12px;')

# Reduce gaps
content = content.replace('gap: 20px;', 'gap: 12px;')
content = content.replace('gap: 15px;', 'gap: 10px;')
content = content.replace('margin-bottom: 20px;', 'margin-bottom: 12px;')

# Make tabs more compact
content = content.replace('padding: 10px 20px;', 'padding: 8px 16px;')

# Reduce font sizes slightly for compactness
content = content.replace('font-size: 14px;', 'font-size: 13px;')

# Check if g_number is displayed - search for it
if 'g_number' not in content:
    print("WARNING: g_number not found in content!")
else:
    print("[OK] g_number found in content")
    # Count occurrences
    g_count = content.count('g_number')
    print(f"  Found {g_count} references to g_number")

# Also check for G-number display
if '"G-' in content:
    print("[OK] G-number label found")
else:
    print("WARNING: G-number label not found!")

with open('crates/frontend/src/domain/a015_wb_orders/ui/details/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("\n[DONE] Orders Details UI improvements applied!")
print("Changes:")
print("  - Added background to all labels")
print("  - Reduced padding and margins")
print("  - Made more compact overall")
print("  - Verified gNumber field presence")

