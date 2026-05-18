#!/usr/bin/env python3
"""
Script to run all candle pattern examples and verify the last match corresponds to the pattern name.
Tests examples from onebar, twobar, threebar, and fourbar subdirectories.
"""

import os
import re
import subprocess
from pathlib import Path


def normalize_name(name):
    """Normalize a name by converting to lowercase and removing spaces/underscores."""
    # Convert numbers to words and vice versa
    replacements = {
        "2": "two",
        "3": "three",
        "4": "four",
        "two": "2",
        "three": "3",
        "four": "4",
    }
    normalized = name.lower().replace("_", "").replace(" ", "").replace("-", "")
    # Also handle bear/bearish and bull/bullish variations
    normalized = normalized.replace("bearish", "bear").replace("bullish", "bull")
    return normalized


def extract_pattern_from_filename(filename):
    """Extract the expected pattern name from the filename."""
    # Remove ti_cdl prefix and _example suffix
    name = filename.replace("ti_cdl", "").replace("_example", "")
    return name


def run_example(example_name):
    """Run a single example and return its output."""
    cmd = ["cargo", "run", "--release", "--example", example_name]
    try:
        result = subprocess.run(
            cmd, cwd=".", capture_output=True, text=True, timeout=30
        )
        return result.returncode, result.stdout + result.stderr
    except subprocess.TimeoutExpired:
        return -1, "TIMEOUT"


def extract_patterns_by_section(output):
    """Extract pattern names from both runs in the output.

    The examples only print patterns from result.last(), so every pattern
    returned here belongs to the same (last) bar position.  A single bar
    position may match multiple patterns when the engine is running in
    all-matches mode.

    Returns:
        tuple: (first_run_patterns, second_run_patterns)
               Each is a list of all pattern names found at the last bar.
    """
    # Split output into sections by looking for "Result:" headers
    # First section is the unfiltered run, second section is the filtered run
    sections = output.split("Result:")

    first_run_patterns = []
    second_run_patterns = []

    if len(sections) > 1:
        # First run patterns (after first "Result:")
        first_matches = re.findall(r"  - ([^(]+)\s*\([^)]+\),\s*Bars:", sections[1])
        first_run_patterns = [p.strip() for p in first_matches]

    if len(sections) > 2:
        # Second run patterns (after second "Result:")
        second_matches = re.findall(r"  - ([^(]+)\s*\([^)]+\),\s*Bars:", sections[2])
        second_run_patterns = [p.strip() for p in second_matches]

    return first_run_patterns, second_run_patterns


def main():
    # Find all candle example files in all subdirectories
    examples_base = Path("tulip_rs/examples/candle")
    if not examples_base.exists():
        examples_base = Path("examples/candle")

    # Collect all example files from all subdirectories
    example_files = []
    for subdir in ["onebar", "twobar", "threebar", "fourbar"]:
        subdir_path = examples_base / subdir
        if subdir_path.exists():
            example_files.extend(sorted(subdir_path.glob("*.rs")))

    if not example_files:
        print("❌ No example files found!")
        return 1

    # Special cases: examples that intentionally show different patterns
    # These demonstrate extended patterns or alternative names
    special_cases = {
        "ti_cdlbearishengulfing_example": "Extended pattern demonstration (Three Outside Down)",
    }

    print("=" * 80)
    print("CANDLE PATTERN EXAMPLES TEST")
    print("=" * 80)
    print(f"Found {len(example_files)} examples to test")
    print()

    passed = []
    failed = []
    skipped = []

    for example_file in example_files:
        example_name = example_file.stem  # filename without .rs extension
        expected_pattern_part = extract_pattern_from_filename(example_name)
        subdir = example_file.parent.name

        print(f"Testing: {example_name} ({subdir})")

        # Check if this is a special case
        if example_name in special_cases:
            print(f"  ⚠️  SKIPPED: {special_cases[example_name]}")
            skipped.append((example_name, subdir, special_cases[example_name]))
            print()
            continue

        print(f"  Expected pattern contains: {expected_pattern_part}")

        returncode, output = run_example(example_name)

        if returncode != 0:
            print(f"  ❌ FAILED: Example failed to run (exit code: {returncode})")
            failed.append((example_name, subdir, "failed to run"))
            print()
            continue

        first_run_patterns, second_run_patterns = extract_patterns_by_section(output)

        # Check first run (no filtering)
        if not first_run_patterns:
            print(f"  ❌ FAILED: No pattern matches found in first run (no filtering)")
            failed.append((example_name, subdir, "no matches in first run"))
            print()
            continue

        # Check second run (with forecast filtering)
        if not second_run_patterns:
            print(
                f"  ❌ FAILED: No pattern matches found in second run (with filtering)"
            )
            failed.append((example_name, subdir, "no matches in second run"))
            print()
            continue

        print(f"  First run last-bar patterns:  {first_run_patterns}")
        print(f"  Second run last-bar patterns: {second_run_patterns}")

        # Normalize expected name for comparison
        normalized_expected = normalize_name(expected_pattern_part)
        expected_with_words = (
            normalized_expected.replace("2", "two")
            .replace("3", "three")
            .replace("4", "four")
        )
        expected_with_nums = (
            normalized_expected.replace("two", "2")
            .replace("three", "3")
            .replace("four", "4")
        )

        def expected_in(pattern_name):
            """Return True if the expected pattern name is contained in this pattern name."""
            n = normalize_name(pattern_name)
            return (
                normalized_expected in n
                or expected_with_words in n
                or expected_with_nums in n
            )

        # Pass if the expected pattern appears in ANY of the last-bar patterns
        first_match_found = any(expected_in(p) for p in first_run_patterns)
        second_match_found = any(expected_in(p) for p in second_run_patterns)

        # Both runs must pass
        if first_match_found and second_match_found:
            print(f"  ✅ PASSED (both runs)")
            passed.append((example_name, subdir))
        else:
            if not first_match_found:
                print(
                    f"  ❌ FAILED: First run — '{normalized_expected}' not found in any last-bar pattern"
                )
                print(f"     Expected (normalized): {normalized_expected}")
                print(
                    f"     Actual (normalized):   {[normalize_name(p) for p in first_run_patterns]}"
                )
                failed.append(
                    (
                        example_name,
                        subdir,
                        f"first run missing pattern: {first_run_patterns}",
                    )
                )
            if not second_match_found:
                print(
                    f"  ❌ FAILED: Second run — '{normalized_expected}' not found in any last-bar pattern"
                )
                print(f"     Expected (normalized): {normalized_expected}")
                print(
                    f"     Actual (normalized):   {[normalize_name(p) for p in second_run_patterns]}"
                )
                failed.append(
                    (
                        example_name,
                        subdir,
                        f"second run missing pattern: {second_run_patterns}",
                    )
                )

        print()

    # Print summary
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"Passed: {len(passed)}/{len(example_files)}")
    print(f"Failed: {len(failed)}/{len(example_files)}")
    print(f"Skipped: {len(skipped)}/{len(example_files)} (special cases)")
    print()

    if passed:
        print("Passed examples by category:")
        by_category = {}
        for name, subdir in passed:
            by_category.setdefault(subdir, []).append(name)
        for category in sorted(by_category.keys()):
            print(f"  {category}: {len(by_category[category])}")
        print()

    if skipped:
        print("Skipped examples (special cases):")
        for name, subdir, reason in skipped:
            print(f"  - {name} ({subdir}): {reason}")
        print()

    if failed:
        print("Failed examples:")
        for name, subdir, reason in failed:
            print(f"  - {name} ({subdir}): {reason}")
        print()
        return 1
    else:
        print("All examples passed! ✅")
        return 0


if __name__ == "__main__":
    exit(main())
