.PHONY: all clean view

TEX = whitepaper.tex
PDF = whitepaper.pdf

all: $(PDF)

$(PDF): $(TEX)
	latexmk -pdf -interaction=nonstopmode $(TEX)

clean:
	latexmk -C
	rm -f *.aux *.log *.out *.toc *.lof *.lot *.bbl *.blg \
	      *.synctex.gz *.fls *.fdb_latexmk _minted-*

view: $(PDF)
	open $(PDF)

watch:
	latexmk -pdf -pvc $(TEX)
