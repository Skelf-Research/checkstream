#!/usr/bin/env python3
"""Build tokenizer.json from vocab.txt for BERT models"""

import json
import sys
from pathlib import Path


def build_tokenizer_json(model_dir):
    """Build a tokenizer.json file from vocab.txt"""
    model_path = Path(model_dir)
    vocab_path = model_path / "vocab.txt"

    if not vocab_path.exists():
        print(f"Error: {vocab_path} not found")
        sys.exit(1)

    # Read vocab
    with open(vocab_path) as f:
        vocab = [line.strip() for line in f]

    # Create vocab dict
    vocab_dict = {word: idx for idx, word in enumerate(vocab)}

    # Build tokenizer.json structure
    tokenizer_json = {
        "version": "1.0",
        "truncation": None,
        "padding": None,
        "added_tokens": [
            {"id": 0, "content": "[PAD]", "single_word": False, "lstrip": False, "rstrip": False, "normalized": False, "special": True},
            {"id": 100, "content": "[UNK]", "single_word": False, "lstrip": False, "rstrip": False, "normalized": False, "special": True},
            {"id": 101, "content": "[CLS]", "single_word": False, "lstrip": False, "rstrip": False, "normalized": False, "special": True},
            {"id": 102, "content": "[SEP]", "single_word": False, "lstrip": False, "rstrip": False, "normalized": False, "special": True},
            {"id": 103, "content": "[MASK]", "single_word": False, "lstrip": False, "rstrip": False, "normalized": False, "special": True},
        ],
        "normalizer": {
            "type": "BertNormalizer",
            "clean_text": True,
            "handle_chinese_chars": True,
            "strip_accents": None,
            "lowercase": True
        },
        "pre_tokenizer": {
            "type": "BertPreTokenizer"
        },
        "post_processor": {
            "type": "TemplateProcessing",
            "single": [
                {"SpecialToken": {"id": "[CLS]", "type_id": 0}},
                {"Sequence": {"id": "A", "type_id": 0}},
                {"SpecialToken": {"id": "[SEP]", "type_id": 0}}
            ],
            "pair": [
                {"SpecialToken": {"id": "[CLS]", "type_id": 0}},
                {"Sequence": {"id": "A", "type_id": 0}},
                {"SpecialToken": {"id": "[SEP]", "type_id": 0}},
                {"Sequence": {"id": "B", "type_id": 1}},
                {"SpecialToken": {"id": "[SEP]", "type_id": 1}}
            ],
            "special_tokens": {
                "[CLS]": {"id": "[CLS]", "ids": [101], "tokens": ["[CLS]"]},
                "[SEP]": {"id": "[SEP]", "ids": [102], "tokens": ["[SEP]"]}
            }
        },
        "decoder": {
            "type": "WordPiece",
            "prefix": "##",
            "cleanup": True
        },
        "model": {
            "type": "WordPiece",
            "unk_token": "[UNK]",
            "continuing_subword_prefix": "##",
            "max_input_chars_per_word": 100,
            "vocab": vocab_dict
        }
    }

    # Write tokenizer.json
    output_path = model_path / "tokenizer.json"
    with open(output_path, 'w') as f:
        json.dump(tokenizer_json, f, indent=2)

    print(f"✓ Created {output_path}")
    print(f"  Vocab size: {len(vocab)}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: build_tokenizer.py <model_directory>")
        sys.exit(1)

    build_tokenizer_json(sys.argv[1])
