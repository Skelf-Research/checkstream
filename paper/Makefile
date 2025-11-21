# Makefile for CheckStream research paper
# Author: Dipankar Sarkar

# Main document
MAIN = checkstream
TEX = $(MAIN).tex
PDF = $(MAIN).pdf
BIB = $(MAIN).bib

# LaTeX compiler
LATEX = pdflatex
BIBTEX = bibtex

# Flags
LATEX_FLAGS = -interaction=nonstopmode -halt-on-error

# Default target
all: $(PDF)

# Build PDF with bibliography
$(PDF): $(TEX)
	@echo "Building $(PDF)..."
	$(LATEX) $(LATEX_FLAGS) $(TEX)
	$(LATEX) $(LATEX_FLAGS) $(TEX)
	@echo "Build complete: $(PDF)"

# Build with bibliography (if using .bib file)
bib: $(TEX)
	@echo "Building with bibliography..."
	$(LATEX) $(LATEX_FLAGS) $(TEX)
	$(BIBTEX) $(MAIN)
	$(LATEX) $(LATEX_FLAGS) $(TEX)
	$(LATEX) $(LATEX_FLAGS) $(TEX)
	@echo "Build complete: $(PDF)"

# Quick build (single pass)
quick:
	@echo "Quick build..."
	$(LATEX) $(LATEX_FLAGS) $(TEX)

# Clean auxiliary files
clean:
	@echo "Cleaning auxiliary files..."
	rm -f *.aux *.log *.bbl *.blg *.toc *.out *.lot *.lof *.fls *.fdb_latexmk *.synctex.gz
	@echo "Clean complete"

# Clean everything including PDF
cleanall: clean
	@echo "Removing PDF..."
	rm -f $(PDF)
	@echo "Full clean complete"

# Watch for changes and rebuild (requires inotifywait)
watch:
	@echo "Watching for changes..."
	@while true; do \
		inotifywait -e modify $(TEX); \
		make quick; \
	done

# View PDF (Linux)
view: $(PDF)
	@echo "Opening $(PDF)..."
	@if command -v xdg-open > /dev/null; then \
		xdg-open $(PDF); \
	elif command -v open > /dev/null; then \
		open $(PDF); \
	else \
		echo "No PDF viewer found"; \
	fi

# Word count (approximate)
wordcount:
	@echo "Approximate word count:"
	@detex $(TEX) | wc -w

# Check for common LaTeX issues
check:
	@echo "Checking for common issues..."
	@grep -n "TODO" $(TEX) || echo "No TODOs found"
	@grep -n "FIXME" $(TEX) || echo "No FIXMEs found"
	@grep -n "\\\\ref{.*}" $(TEX) | grep -v "\\\\autoref" || true
	@echo "Check complete"

# Create archive for submission
archive: clean $(PDF)
	@echo "Creating submission archive..."
	tar -czvf $(MAIN)-submission.tar.gz $(TEX) $(PDF) Makefile
	@echo "Archive created: $(MAIN)-submission.tar.gz"

# Help
help:
	@echo "CheckStream Paper Makefile"
	@echo "=========================="
	@echo ""
	@echo "Targets:"
	@echo "  all       - Build PDF (default)"
	@echo "  quick     - Quick build (single pass)"
	@echo "  bib       - Build with bibliography"
	@echo "  clean     - Remove auxiliary files"
	@echo "  cleanall  - Remove all generated files"
	@echo "  view      - Open PDF in viewer"
	@echo "  watch     - Watch for changes and rebuild"
	@echo "  wordcount - Approximate word count"
	@echo "  check     - Check for TODOs and issues"
	@echo "  archive   - Create submission archive"
	@echo "  help      - Show this help"
	@echo ""
	@echo "Usage: make [target]"

.PHONY: all bib quick clean cleanall watch view wordcount check archive help
