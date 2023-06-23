package main

import (
	"encoding/csv"
	"fmt"
	"html/template"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

type table struct {
	headers           []string
	columnNameToIndex map[string]int
}

func newTable(headers []string) *table {
	t := &table{headers: headers, columnNameToIndex: map[string]int{}}

	for i, h := range headers {
		t.columnNameToIndex[h] = i
	}
	return t
}

func (t *table) getRowValue(row []string, columnName string) string {
	if index, ok := t.columnNameToIndex[columnName]; ok {
		return row[index]
	}
	panic("could not find column: " + columnName)
}

// dimensionAssignment contains a assignment of dimensions and values, which is used to
// identify a single row or experiment.
type dimensionAssignment struct {
	names  []string
	values []string
}

// dimensionDictionary contains all the values ever seen for a single dimension.
type dimensionDictionary struct {
	name   string
	values stringSet
}

func newDimensionDictionary(name string) *dimensionDictionary {
	return &dimensionDictionary{name, *newStringSet()}
}

func main() {
	fmt.Println("reading csv from stdin...")

	const xAxisHeader = "read_buffer_bytes"
	const yAxisHeader = "throughput (MiB/s)"

	dimensionNames := []string{
		"machine_configuration", "use_buffer", "write_buffer_bytes", "connection_type",
	}

	filters := []struct {
		columnName string
		value      string
	}{
		{"machine_configuration", "intel_i5_11thgen_localhost"},
		{"use_buffer", "false"},
		{"connection_type", "unix"},
	}

	input := csv.NewReader(os.Stdin)
	headers, err := input.Read()
	if err != nil {
		panic(err)
	}

	plots := map[dimensionAssignment][]dataPoint{}

	dimensionDictionaries := make([]*dimensionDictionary, len(dimensionNames))
	for i, dimensionName := range dimensionNames {
		dimensionDictionaries[i] = newDimensionDictionary(dimensionName)
	}

	table := newTable(headers)
nextRow:
	for {
		row, err := input.Read()
		if err != nil {
			if err == io.EOF {
				break
			}
			panic(err)
		}

		for _, filter := range filters {
			v := table.getRowValue(row, filter.columnName)
			if v != filter.value {
				continue nextRow
			}
		}

		label := ""
		for _, dimension := range dimensionNames {
			if len(label) > 0 {
				label += " "
			}
			label += fmt.Sprintf("%s=%s", dimension, table.getRowValue(row, dimension))
		}

		xValue := table.getRowValue(row, xAxisHeader)
		yValue := table.getRowValue(row, yAxisHeader)
		plots[label] = append(plots[label], dataPoint{xValue, yValue})
	}

	fmt.Printf("%d distinct data sets\n", len(plots))
	for label := range plots {
		fmt.Printf("  %s\n", label)
	}

	chart := chartDetails{
		title:      "example title",
		xLabel:     xAxisHeader,
		yLabel:     yAxisHeader,
		plots:      plots,
		pathPrefix: "results/plot",
	}
	err = writeGnuplot(chart)
	if err != nil {
		panic(err)
	}

	err = writePlotlyHTML(chart)
	if err != nil {
		panic(err)
	}
}

type dataPoint struct {
	x string
	y string
}

type chartDetails struct {
	title      string
	xLabel     string
	yLabel     string
	plots      map[string][]dataPoint
	pathPrefix string
}

var nonAlphaRE = regexp.MustCompile(`[^a-zA-Z0-9_]+`)

func toCSVFileName(label string) string {
	// force lowercase
	label = strings.ToLower(label)

	// replace any runs of non-alphanumeric ascii with -
	label = nonAlphaRE.ReplaceAllString(label, "-")
	return label
}

func writeCSV(path string, xLabel string, yLabel string, data []dataPoint) error {
	f, err := os.OpenFile(path, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0600)
	if err != nil {
		return err
	}
	defer f.Close()

	writer := csv.NewWriter(f)
	err = writer.Write([]string{xLabel, yLabel})
	if err != nil {
		return err
	}
	for _, row := range data {
		err = writer.Write([]string{row.x, row.y})
		if err != nil {
			return err
		}
	}
	writer.Flush()
	return writer.Error()
}

func writeGnuplot(chart chartDetails) error {
	plotLabelsToCSVPath := map[string]string{}
	for plotLabel, data := range chart.plots {
		csvPath := chart.pathPrefix + "-data-" + toCSVFileName(plotLabel) + ".csv"
		plotLabelsToCSVPath[plotLabel] = csvPath
		err := writeCSV(csvPath, chart.xLabel, chart.yLabel, data)
		if err != nil {
			return err
		}
	}

	gnuplotPath := chart.pathPrefix + ".gnuplot"
	f, err := os.OpenFile(gnuplotPath, os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0600)
	if err != nil {
		return err
	}
	defer f.Close()

	_, err = f.WriteString(gnuplotHeader)
	if err != nil {
		return err
	}

	relativeOutputPlotPath := filepath.Base(chart.pathPrefix + ".pdf")
	_, err = fmt.Fprintf(f, `
set xlabel "%s"
set ylabel "%s"
set title "%s"
set output "%s"
plot \
`, gnuplotEscape(chart.xLabel), gnuplotEscape(chart.yLabel), gnuplotEscape(chart.title), relativeOutputPlotPath)
	if err != nil {
		return err
	}

	first := true
	for plotLabel := range chart.plots {
		if !first {
			_, err = f.WriteString(", \\\n")
			if err != nil {
				return err
			}
		} else {
			first = false
		}
		relativeDataFileName := filepath.Base(plotLabelsToCSVPath[plotLabel])
		_, err = fmt.Fprintf(f, `  "%s" using 1:2 with linespoints title "%s"`,
			relativeDataFileName, gnuplotEscape(plotLabel))
		if err != nil {
			return err
		}
	}
	return nil
}

const gnuplotHeader = `# Note you need gnuplot 4.4 for the pdfcairo terminal.
set terminal pdfcairo enhanced font "Helvetica,6" linewidth 1.0 rounded fontscale 1.0

# Line style for axes
set style line 80 lt rgb "#808080"

# Line style for grid
set style line 81 lt 0	# dashed
set style line 81 lt rgb "#808080"	# grey
set style line 81 linewidth 0.5

set grid back linestyle 81

# Remove border on top and right.	These
# borders are useless and make it harder
# to see plotted lines near the border.
# Also, put it in grey; no need for so much emphasis on a border.
set border 3 back linestyle 80

set xtics nomirror
set ytics nomirror

# Line styles: try to pick pleasing colors, rather
# than strictly primary colors or hard-to-see colors
# like gnuplot's default yellow.	Make the lines thick
# so they're easy to see in small plots in papers.
set style line 1 lt rgb "#A00000" linewidth 2 pointtype 1 pointsize 0.75
set style line 2 lt rgb "#00A000" linewidth 2 pointtype 6 pointsize 0.75
set style line 3 lt rgb "#5060D0" linewidth 2 pointtype 2 pointsize 0.75
set style line 4 lt rgb "#F25900" linewidth 2 pointtype 8 pointsize 0.75
set style line 5 lt rgb "#FF0000" linewidth 2 pointtype 3 pointsize 0.75

set datafile separator ','
set key top left
`

func gnuplotEscape(in string) string {
	// underscore needs to be double escaped
	// https://stackoverflow.com/questions/13655048/display-underscore-rather-than-subscript-in-gnuplot-titles
	return strings.ReplaceAll(in, "_", `\\\_`)
}

type stringSet struct {
	set map[string]struct{}
}

func newStringSet() *stringSet {
	return &stringSet{make(map[string]struct{})}
}

func (s *stringSet) add(str string) {
	s.set[str] = struct{}{}
}

func (s *stringSet) list() []string {
	out := make([]string, 0, len(s.set))
	for str := range s.set {
		out = append(out, str)
	}
	sort.Strings(out)
	return out
}

func numericStringLess(i string, j string) bool {
	iInt, err := strconv.Atoi(i)
	if err != nil {
		panic(err)
	}
	jInt, err := strconv.Atoi(j)
	if err != nil {
		panic(err)
	}

	return iInt < jInt
}

type plotlyTrace struct {
	Name string    `json:"name"`
	X    []int64   `json:"x"`
	Y    []float64 `json:"y"`
}

type plotlyPlot struct {
	Title  string
	XLabel string
	YLabel string
	Traces []*plotlyTrace
}

func writePlotlyHTML(chart chartDetails) error {
	plotly := &plotlyPlot{
		Title:  chart.title,
		XLabel: chart.xLabel,
		YLabel: chart.yLabel,
	}
	for label, dataPoints := range chart.plots {
		trace := &plotlyTrace{Name: label}
		for _, dataPoint := range dataPoints {
			ival, err := strconv.ParseInt(dataPoint.x, 10, 64)
			if err != nil {
				return err
			}
			trace.X = append(trace.X, ival)
			fval, err := strconv.ParseFloat(dataPoint.y, 64)
			if err != nil {
				return err
			}
			trace.Y = append(trace.Y, fval)
		}
		plotly.Traces = append(plotly.Traces, trace)
	}

	outF, err := os.OpenFile(chart.pathPrefix+".html", os.O_CREATE|os.O_TRUNC|os.O_WRONLY, 0600)
	if err != nil {
		return err
	}
	defer outF.Close()
	err = htmlPlotTemplate.Execute(outF, plotly)
	if err != nil {
		return err
	}
	return outF.Close()
}

var htmlPlotTemplate = template.Must(template.New("html").Parse(`<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>{{.Title}}</title>
<script src="https://cdn.plot.ly/plotly-2.24.1.min.js"></script>
<script>
function onLoad() {
	var data = {{.Traces}};
	var layout = {
		title: {{.Title}},
		xaxis: {
			title: {{.XLabel}},
		},
		yaxis: {
			title: {{.YLabel}},
		}
	}

	Plotly.newPlot("plot", data, layout);
}
window.addEventListener("load", onLoad);
</script>
</head>
<body>
<div id="plot" style="width: 100%; height: 800px;"></div>
</body>
</html>
`))
