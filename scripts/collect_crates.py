#!/usr/bin/env python3
"""
Скрипт для сбора всех файлов кода из каталога crates/ в один текстовый файл.
Оптимизирован для анализа LLM.
"""

import os
import sys
from pathlib import Path
from datetime import datetime
from collections import defaultdict


# Конфигурация
INCLUDED_EXTENSIONS = {'.rs', '.toml', '.md', '.css', '.js', '.html'}
EXCLUDED_DIRS = {'target', 'node_modules', '.git'}
SEPARATOR = '=' * 80


def is_binary_file(file_path):
    """Проверка, является ли файл бинарным."""
    try:
        with open(file_path, 'rb') as f:
            chunk = f.read(1024)
            # Проверяем наличие нулевых байтов
            if b'\x00' in chunk:
                return True
            # Проверяем процент непечатных символов
            text_chars = bytearray({7,8,9,10,12,13,27} | set(range(0x20, 0x100)))
            non_text = len([b for b in chunk if b not in text_chars])
            return non_text / len(chunk) > 0.3 if chunk else False
    except Exception:
        return True


def read_file_content(file_path):
    """Читает содержимое файла с обработкой ошибок кодировки."""
    encodings = ['utf-8', 'utf-8-sig', 'latin-1', 'cp1251']
    
    for encoding in encodings:
        try:
            with open(file_path, 'r', encoding=encoding) as f:
                return f.read(), encoding
        except UnicodeDecodeError:
            continue
        except Exception as e:
            return None, f"Error: {str(e)}"
    
    return None, "Unable to decode file"


def should_process_file(file_path, root_dir):
    """Проверяет, нужно ли обрабатывать файл."""
    # Проверка расширения
    if file_path.suffix.lower() not in INCLUDED_EXTENSIONS:
        return False
    
    # Проверка исключенных директорий
    try:
        rel_path = file_path.relative_to(root_dir)
        for excluded in EXCLUDED_DIRS:
            if excluded in rel_path.parts:
                return False
    except ValueError:
        return False
    
    return True


def collect_files(crates_dir):
    """Собирает все подходящие файлы из директории crates/."""
    files = []
    skipped = []
    stats = defaultdict(int)
    
    crates_path = Path(crates_dir)
    
    if not crates_path.exists():
        print(f"ERROR: Directory not found: {crates_path}")
        return files, skipped, stats
    
    print(f"Scanning {crates_path}...")
    
    # Рекурсивный обход всех файлов
    for file_path in crates_path.rglob('*'):
        if not file_path.is_file():
            continue
        
        if should_process_file(file_path, crates_path):
            # Проверка на бинарный файл
            if is_binary_file(file_path):
                skipped.append((str(file_path), "Binary file"))
                continue
            
            files.append(file_path)
            stats[file_path.suffix.lower()] += 1
            stats['total'] += 1
    
    return files, skipped, stats


def format_file_entry(file_path, project_root, content, encoding):
    """Форматирует запись файла для вывода."""
    try:
        rel_path = file_path.relative_to(project_root)
    except ValueError:
        rel_path = file_path
    
    size = file_path.stat().st_size
    
    entry = f"\n{SEPARATOR}\n"
    entry += f"FILE: {rel_path}\n"
    entry += f"FULL PATH: {file_path}\n"
    entry += f"SIZE: {size} bytes\n"
    entry += f"ENCODING: {encoding}\n"
    entry += f"{SEPARATOR}\n"
    entry += content
    entry += "\n\n"
    
    return entry


def generate_header(stats, crates_dir, skipped):
    """Генерирует заголовок с статистикой."""
    header = f"{SEPARATOR}\n"
    header += "CRATES DIRECTORY DUMP\n"
    header += f"{SEPARATOR}\n"
    header += f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
    header += f"Root Directory: {crates_dir}\n"
    header += f"\n"
    header += f"STATISTICS:\n"
    header += f"- Total files processed: {stats['total']}\n"
    header += f"- Files skipped: {len(skipped)}\n"
    header += f"\n"
    header += f"By extension:\n"
    
    # Сортируем расширения по количеству файлов
    extensions = [(ext, count) for ext, count in stats.items() if ext != 'total']
    extensions.sort(key=lambda x: x[1], reverse=True)
    
    for ext, count in extensions:
        header += f"  - {ext:8} : {count:4} files\n"
    
    if skipped:
        header += f"\n"
        header += f"SKIPPED FILES:\n"
        for file_path, reason in skipped[:10]:  # Показываем первые 10
            header += f"  - {Path(file_path).name}: {reason}\n"
        if len(skipped) > 10:
            header += f"  ... and {len(skipped) - 10} more\n"
    
    header += f"\n{SEPARATOR}\n"
    
    return header


def main():
    """Основная функция скрипта."""
    # Определяем пути
    script_dir = Path(__file__).parent.absolute()
    project_root = script_dir.parent
    crates_dir = project_root / 'crates'
    output_file = script_dir / 'crates_dump.txt'
    
    print("="*60)
    print("CRATES DIRECTORY COLLECTOR")
    print("="*60)
    print(f"Project root: {project_root}")
    print(f"Crates directory: {crates_dir}")
    print(f"Output file: {output_file}")
    print()
    
    # Собираем файлы
    files, skipped, stats = collect_files(crates_dir)
    
    if not files:
        print("ERROR: No files found to process!")
        return 1
    
    print(f"\nFound {len(files)} files to process")
    print(f"Skipped {len(skipped)} files")
    
    # Сортируем файлы по пути для предсказуемого порядка
    files.sort(key=lambda x: str(x))
    
    # Создаем выходной файл
    print(f"\nWriting to {output_file}...")
    
    try:
        with open(output_file, 'w', encoding='utf-8') as out:
            # Пишем заголовок
            header = generate_header(stats, crates_dir, skipped)
            out.write(header)
            
            # Обрабатываем каждый файл
            processed = 0
            errors = []
            
            for file_path in files:
                content, encoding_or_error = read_file_content(file_path)
                
                if content is None:
                    errors.append((str(file_path), encoding_or_error))
                    continue
                
                entry = format_file_entry(file_path, project_root, content, encoding_or_error)
                out.write(entry)
                
                processed += 1
                if processed % 50 == 0:
                    print(f"  Processed {processed}/{len(files)} files...")
            
            if errors:
                out.write(f"\n{SEPARATOR}\n")
                out.write("ERRORS DURING PROCESSING:\n")
                out.write(f"{SEPARATOR}\n")
                for file_path, error in errors:
                    out.write(f"- {Path(file_path).name}: {error}\n")
        
        print(f"\n{SEPARATOR}")
        print(f"SUCCESS!")
        print(f"{SEPARATOR}")
        print(f"Processed: {processed} files")
        print(f"Output file: {output_file}")
        print(f"File size: {output_file.stat().st_size:,} bytes")
        
        if errors:
            print(f"\nWarning: {len(errors)} files had read errors")
        
        return 0
        
    except Exception as e:
        print(f"\nERROR: Failed to write output file: {e}")
        return 1


if __name__ == '__main__':
    sys.exit(main())
