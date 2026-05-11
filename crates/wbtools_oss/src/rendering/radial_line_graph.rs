pub struct RadialLineGraph {
    pub parent_id: String,
    pub width: f64,
    pub height: f64,
    pub data_x: Vec<Vec<f64>>,
    pub data_y: Vec<Vec<f64>>,
    pub series_labels: Vec<String>,
    pub x_axis_label: String,
    pub x_symbol: String,
    pub y_axis_label: String,
    pub y_symbol: String,
    pub draw_points: bool,
    pub draw_gridlines: bool,
    pub draw_legend: bool,
    pub draw_grey_background: bool,
    pub fill_polygons: bool,
}

impl RadialLineGraph {
    pub fn get_svg(&self) -> String {
        let data_x2 = format!("{:?}", self.data_x);
        let data_y2 = format!("{:?}", self.data_y);
        let series_labels2 = format!("{:?}", self.series_labels);
        let mut s = String::new();
        s.push_str(&format!(
            "
    <script>
      var plot = {{
        dataX: {},
        dataY: {},
        seriesLabels: {},
        xAxisLabel: \"{}\",
        xSymbol: \"{}\",
        yAxisLabel: \"{}\",
        ySymbol: \"{}\",
        width: {},
        height: {},
        drawPoints: {},
        drawGridlines: {},
        drawLegend: {},
        drawGreyBackground: {},
        fillPolygons: {},
        parentId: \"{}\"
      }};",
            data_x2,
            data_y2,
            series_labels2,
            self.x_axis_label,
            self.x_symbol,
            self.y_axis_label,
            self.y_symbol,
            self.width,
            self.height,
            self.draw_points,
            self.draw_gridlines,
            self.draw_legend,
            self.draw_grey_background,
            self.fill_polygons,
            self.parent_id
        ));

        s.push_str(&r#"
      function update(svg) {
        // which of the series labels is longest?
        var maxSeriesLabelLength = 0;
        var a;
        for (a = 0; a < plot.seriesLabels.length; a++) {
          var sl = plot.seriesLabels[a];
          if (sl.length > maxSeriesLabelLength) { maxSeriesLabelLength = sl.length; }
        }
        var plotLeftMargin = 70.0;
        var plotRightMargin = plot.drawLegend ? 65.0 + maxSeriesLabelLength * 7 : 50.0;
        var plotBottomMargin = 70.0;
        var plotTopMargin = 40.0;
        var plotWidth = plot.width - plotLeftMargin - plotRightMargin;
        var plotHeight = plot.height - plotBottomMargin - plotTopMargin;
        var originX = plotLeftMargin;
        var originY = plotTopMargin + plotHeight;
        var tickLen = 8.0;
        var minorTickLen = tickLen * 0.65;

        // If there are no series labels, treat it as one series.
        if (plot.seriesLabels.length === 0) {
					plot.drawLegend = false;
				}

        // colors (modernized Whitebox-inspired palette)
        var lineColor = '#2c7fb8';
        var highlightColor = '#f28e2b';
        var btnColor = '#4b5563';
        var btnHoverColor = '#374151';
        var plotBackgroundColor = '#f8fafc';
        if (plot.drawGreyBackground) {
          plotBackgroundColor = '#CCC';
        }
        var chartBackgroundColor = '#ffffff';
        // var gridLineColor = 'rgb(120,120,120)';
        // if (plot.drawGreyBackground) {
          var gridLineColor = '#d6deea';
        // }
        var trendlineColor = 'DimGray';
        if (plot.drawGreyBackground) {
          trendlineColor = 'DimGray';
        }
        var showValueClr = '#334155';
        var axisColor = '#334155';
        // if (plot.drawGreyBackground) {
        //   showValueClr = '#FFF';
        // }

        // Gridlines
        // var gridlineDash = '1, 5';
        // if (plot.drawGreyBackground) {
          var gridlineDash = 'none';
        // }

        var tableau20 = [[31, 119, 180], [255, 127, 14],
             [44, 160, 44], [214, 39, 40],
             [148, 103, 189], [140, 86, 75],
             [227, 119, 194], [127, 127, 127],
             [188, 189, 34], [23, 190, 207]];

        var regularOpacity = 1.0;
        var deselectedOpacity = 0.10;


        // create the svg element
        var svgns = "http://www.w3.org/2000/svg";
        if (svg == null) {
          svg = document.createElementNS(svgns, "svg");
        } else {
          while (svg.lastChild) {
              svg.removeChild(svg.lastChild);
          }
        }
        svg.setAttribute('width', `${plot.width}`);
        svg.setAttribute('height', `${plot.height}`);
        var div = document.getElementById(plot.parentId);
        if (div != null) {
          div.appendChild(svg);
        } else {
          // add it to the body of the document
          document.querySelector("body").appendChild(svg);
        }

        // how many series are there?
        var numSeries = plot.dataY.length;

        // if dataX is empty, fill it with the series 1, 2, 3, 4, ...
        if (plot.dataX.length == 0) {
          for (s = 0; s < numSeries; s++) {
            var seriesXData = [];
            for (a = 0; a < plot.dataY[s].length; a++) {
              seriesXData.push(a + 1);
            }
            plot.dataX.push(seriesXData);
          }
        }

        // style
        var style = document.createElement("style");
        let styleString = `
        text {
          font-family: Helvetica, Arial, sans-serif;
        }
        .axisLabel {
          font-weight: 600;
          fill: ${axisColor};
        }
        .xTickLabel {
          fill: ${axisColor};
          font-size: 80%;
          font-weight: 400;
        }
        .yTickLabel {
          fill: ${axisColor};
          font-size: 80%;
          font-weight: 400;
        }
        .gridLine {
          stroke: ${gridLineColor};
          stroke-dasharray: ${gridlineDash};
          stroke-width: 0.9;
        }
        .tick {
          stroke: ${axisColor};
          stroke-width: 0.8;
        }
        #plotBorder {
          fill: none;
          stroke: ${axisColor};
          stroke-width: 0.8;
        }
        #showValue {
          font-size: 85%;
          fill: ${showValueClr};
        }
        #context-menu {
          position:absolute;
          display:none;
        }
        #context-menu ul {
          list-style:none;
          margin:0;
          padding:0;
          background: #EFEFEF;
          opacity: 0.90;
        }
        #context-menu {
          border:solid 1px #CCC;
        }
        #context-menu li {
          font-family:Sans,Arial;
          font-size: 75%;
          text-align: left;
          color:#000;
          display:block;
          padding:5px 15px;
          border-bottom:solid 1px #CCC;
        }
        #context-menu li:last-child {
          border:none;
        }
        #context-menu li:hover {
          background:#007AFF;
          color:#FFF;
        }
        `;

        var dataPointHoverWidth = 4.0;
        if (!plot.drawPoints) {
          dataPointHoverWidth = 6.0;
        }
        var s;
        for (s = 0; s < numSeries; s++) {
          var clrNum = s % tableau20.length;
          if (plot.seriesLabels.length === 0) {
            // If there are no series labels, treat it as one series.
						clrNum = 0;
					}
          let clr = `rgb(${tableau20[clrNum][0]},${tableau20[clrNum][1]},${tableau20[clrNum][2]})`;
          var fill_clr = `none`;
          if (plot.fillPolygons === true) {
            fill_clr = `rgb(${(255 - tableau20[clrNum][0])*0.75+tableau20[clrNum][0]},${(255-tableau20[clrNum][1])*0.75+tableau20[clrNum][1]},${(255-tableau20[clrNum][2])*0.75+tableau20[clrNum][2]})`;
          }
          let fill_opacity = 0.2;

          styleString += `
          .seriesLine${s} {
            fill: ${fill_clr};
            fill-opacity: ${fill_opacity};
            stroke-width:1.6;
            stroke: ${clr};
            opacity: ${regularOpacity};
          }
          .seriesLine${s}:hover {
            fill: ${fill_clr};
            fill-opacity: ${fill_opacity};
            stroke-width:2.4;
            stroke: ${clr};
            opacity: ${regularOpacity};
          }
          .seriesLineThick${s} {
            fill: ${fill_clr};
            fill-opacity: ${fill_opacity};
            stroke-width:2.4;
            stroke: ${clr};
            opacity: ${regularOpacity};
          }
          .seriesLineThick${s}:hover {
            fill: ${fill_clr};
            fill-opacity: ${fill_opacity};
            stroke-width:3.0;
            stroke: ${clr};
            opacity: ${regularOpacity};
          }
          .dataPoint${s} {
            fill: ${clr};
            stroke-width:0;
            stroke: ${clr};
            opacity: ${regularOpacity};
          }
          .dataPoint${s}:hover {
            fill: red;
            stroke-width:${dataPointHoverWidth};
            stroke: red;
            opacity:1.0;
          }
          `;
        }
        style.innerHTML = styleString;
        svg.appendChild(style);
        svg.id = "plotSvg${plot.parentId}";

        // background
        var background = document.createElementNS(svgns, "rect");
        background.setAttribute('width', plot.width);
        background.setAttribute('height', plot.height);
        background.style.fill = chartBackgroundColor;
        svg.appendChild(background);

        // translate the origin point
        var g = document.createElementNS(svgns, "g");
        g.setAttribute('id', 'transform');
        g.setAttribute('transform', `translate(${originX},${originY})`);
        svg.appendChild(g);

        var plotRadius = plotHeight / 2.0;

        // plot background
        var plotBackground = document.createElementNS(svgns, "circle");
        plotBackground.setAttribute('id', 'plotBackground');
        plotBackground.setAttribute('cx', plotWidth/2);
        plotBackground.setAttribute('cy', -plotHeight/2);
        plotBackground.setAttribute('r', plotRadius);
        // plotBackground.setAttribute('height', plotHeight);
        plotBackground.style.fill = plotBackgroundColor;
        plotBackground.style.stroke = '#e3eaf2';
        plotBackground.style.strokeWidth = 0.8;
        g.appendChild(plotBackground);

        // what are the min/max values?
        var xMin = Infinity;
        var xMax = -Infinity;
        var yMin = Infinity;
        var yMax = -Infinity;
        var val = 0;
        var maxNumPoints = 0;
        var totalNumPoints = 0;
        for (s = 0; s < numSeries; s++) {
          var numPoints = Math.min(plot.dataX[s].length, plot.dataY[s].length);
          if (numPoints > maxNumPoints) { maxNumPoints = maxNumPoints; }
          totalNumPoints += numPoints;
          if (numPoints < 2) {
            alert("Too few points for line graph");
            return;
          }
          if (plot.dataX[s].length != plot.dataY[s].length) {
            alert("The x and y data arrays are unequal in length.");
          }
          for (a = 0; a < numPoints; a++) {
              val = plot.dataX[s][a];
              if (val < xMin) { xMin = val; }
              if (val > xMax) { xMax = val; }
              val = plot.dataY[s][a];
              if (val < yMin) { yMin = val; }
              if (val > yMax) { yMax = val; }
          }
        }

        var slopeIncrement = 50.0;
        var numSlopeTicks = yMax / slopeIncrement;
        if (numSlopeTicks < 3 || numSlopeTicks > 8) {
          var possibleIncrements = [1000.0, 500.0, 250.0, 100.0, 50.0, 20.0, 15.0, 10.0, 8.0, 5.0, 4.0, 2.0, 1.0, 0.8, 0.5, 0.4, 0.2, 0.1, 0.05];
          for (let i = 0; i < possibleIncrements.length; i++) { 
            slopeIncrement = possibleIncrements[i];
            numSlopeTicks = yMax / slopeIncrement;
            if (numSlopeTicks >= 3) {
              break;
            }
          }
        }

        yMax = Math.ceil(yMax / slopeIncrement) * slopeIncrement;

        var pi = Math.PI;
        var degToRad = pi / 180.0;

        var angle = 0.0;
        while (angle < 360.0) {
          var line = document.createElementNS(svgns, "line");
          x = plotRadius * Math.cos(angle * degToRad) + plotWidth / 2.0;
          y = plotRadius * Math.sin(angle * degToRad) - plotHeight / 2.0;

          line.setAttribute('x1', plotWidth / 2.0);
          line.setAttribute('y1', -plotHeight / 2.0);
          line.setAttribute('x2', x);
          line.setAttribute('y2', y);
          line.setAttribute('class', 'gridLine');
          g.appendChild(line);

          // axis labels
          x = (plotRadius + 18) * Math.cos(angle * degToRad) + plotWidth / 2.0;
          y = (plotRadius + 18) * Math.sin(angle * degToRad) - plotHeight / 2.0;
          var labelAngle = angle;
          labelAngle += 90.0
          if (labelAngle < 360) { labelAngle += 360; }
          if (labelAngle >= 360) { labelAngle -= 360; }
          var xLabel = document.createElementNS(svgns, "text");
          xLabel.setAttribute('x', x);
          xLabel.setAttribute('y', y);
          xLabel.setAttribute('text-anchor', 'middle');
          xLabel.setAttribute('dominant-baseline', 'middle');
          xLabel.setAttribute('class', 'xTickLabel');
          if (labelAngle == 0) {
            xLabel.innerHTML = `${plot.xSymbol}=${labelAngle.toFixed(0)}`;
          } else {
            xLabel.innerHTML = labelAngle.toFixed(0);
          }
          g.appendChild(xLabel);

          angle += 30.0;
        }

        var slope = 0.0;
        while (slope < yMax) {
          var plotGridLine = document.createElementNS(svgns, "circle");
          // plotGridLine.setAttribute('id', 'plotBackground');
          plotGridLine.setAttribute('cx', plotWidth/2);
          plotGridLine.setAttribute('cy', -plotHeight/2);
          plotGridLine.setAttribute('r', slope / yMax * plotRadius);
          plotGridLine.setAttribute('class', 'gridLine');
          plotGridLine.style.fill = "none";
          g.appendChild(plotGridLine);

          var yLabel = document.createElementNS(svgns, "text");
          yLabel.setAttribute('x', plotWidth/2);
          yLabel.setAttribute('y', -plotHeight/2 - slope / yMax * plotRadius - 5);
          yLabel.setAttribute('text-anchor', 'middle');
          yLabel.setAttribute('dominant-baseline', 'hanging');
          yLabel.setAttribute('class', 'yTickLabel');
          // yLabel.setAttribute('fill', gridLineColor);
          if (slope == 0) {
            yLabel.innerHTML = `${plot.ySymbol}=${slope.toFixed(1)}`;
          } else {
            yLabel.innerHTML = slope.toFixed(1);
          }
          g.appendChild(yLabel);

          slope += slopeIncrement;
        }

        // text to show values when hover over
        var showValue = document.createElementNS(svgns, "text");
        showValue.setAttribute('id', 'showValue_${plot.parentId}');
        showValue.setAttribute('x', 10);
        showValue.setAttribute('y', -plotHeight - 10);
        showValue.setAttribute('text-anchor', 'start');

        var rect = document.createElementNS(svgns, 'rect');
        rect.setAttribute('id', 'rect_${plot.parentId}');
        rect.setAttribute('x', 10);
        rect.setAttribute('y', -plotHeight - 10);
        rect.setAttribute('width', 0);
        rect.setAttribute('height', 0);
        rect.setAttribute('fill', "white");
        g.appendChild(rect);

        var plt = plot;


        // draw the line(s)
        var g2 = document.createElementNS(svgns, "g");
        g2.setAttribute('id', 'lines');
        g.appendChild(g2);

        var radius = 3.0;
        if (totalNumPoints > 15) { radius = 2.5; }
        if (!plot.drawPoints) {
          radius = 1.0;
        }

        for (let s = 0; s < numSeries; s++) {
          var numPoints = Math.min(plot.dataX[s].length, plot.dataY[s].length);
          let seriesLine = document.createElementNS(svgns, "polyline");
          // seriesLine.setAttribute('style', 'fill:lightblue;fill-opacity:0.2');
          var angle = plot.dataX[s][0];
          angle -= 90.0
          if (angle < 360) { angle += 360; }
          if (angle > 360) { angle -= 360; }
          angle *= degToRad;
          var x = plot.dataY[s][0] / yMax * plotRadius * Math.cos(angle) + plotWidth / 2.0;
          var y = plot.dataY[s][0] / yMax * plotRadius * Math.sin(angle) - plotHeight / 2.0;
          var pointsString = `${x},${y}`;
          for (let a = 1; a < numPoints; a++) {
            angle = plot.dataX[s][a];
            angle -= 90.0
            if (angle < 360) { angle += 360; }
            if (angle > 360) { angle -= 360; }
            angle *= degToRad;
            x = plot.dataY[s][a] / yMax * plotRadius * Math.cos(angle) + plotWidth / 2.0;
            y = plot.dataY[s][a] / yMax * plotRadius * Math.sin(angle) - plotHeight / 2.0;
            pointsString += ` ${x},${y}`;
          }
          angle = plot.dataX[s][0];
          angle -= 90.0
          if (angle < 360) { angle += 360; }
          if (angle > 360) { angle -= 360; }
          angle *= degToRad;
          x = plot.dataY[s][0] / yMax * plotRadius * Math.cos(angle) + plotWidth / 2.0;
          y = plot.dataY[s][0] / yMax * plotRadius * Math.sin(angle) - plotHeight / 2.0;
          pointsString += ` ${x},${y}`;
          seriesLine.setAttribute('points', pointsString);
          var seriesLabel = "seriesLine";
          if (!plot.drawPoints) { seriesLabel = "seriesLineThick"; }
          seriesLine.setAttribute('class', `${seriesLabel}${s}`);
          seriesLine.addEventListener('mouseover', function() {
            var s2;
            for (s2 = 0; s2 < numSeries; s2++) {
              if (s2 != s) {
                var x = document.getElementsByClassName(`${seriesLabel}${s2}`);
                var i;
                for (i = 0; i < x.length; i++) {
                  x[i].style.opacity = deselectedOpacity;
                }
                x = document.getElementsByClassName(`dataPoint${s2}`);
                for (i = 0; i < x.length; i++) {
                    x[i].style.opacity = deselectedOpacity;
                }
              }
            }
          }, false);
          seriesLine.addEventListener('mouseout', function() {
            var s2;
            for (s2 = 0; s2 < numSeries; s2++) {
              if (s2 != s) {
                var x = document.getElementsByClassName(`${seriesLabel}${s2}`);
                var i;
                for (i = 0; i < x.length; i++) {
                  x[i].style.opacity = regularOpacity;
                }
                x = document.getElementsByClassName(`dataPoint${s2}`);
                for (i = 0; i < x.length; i++) {
                    x[i].style.opacity = regularOpacity;
                }
              }
            }
          }, false);
          g2.appendChild(seriesLine);

          // draw the data points
          for (let a = 0; a < numPoints; a++) {
            let c = document.createElementNS(svgns, "circle");
            var angle = plot.dataX[s][a];
            angle -= 90.0
            if (angle < 360) { angle += 360; }
            if (angle > 360) { angle -= 360; }
            angle *= degToRad;
            var x = plot.dataY[s][a] / yMax * plotRadius * Math.cos(angle) + plotWidth / 2.0;
            var y = plot.dataY[s][a] / yMax * plotRadius * Math.sin(angle) - plotHeight / 2.0;
            c.setAttribute('cx', `${x}`);
            c.setAttribute('cy', `${y}`);
            c.setAttribute('r', radius);
            c.setAttribute('class', `dataPoint${s}`);
            c.addEventListener('mouseover', function() {
              var s2;
              for (s2 = 0; s2 < numSeries; s2++) {
                if (s2 != s) {
                  var x = document.getElementsByClassName(`${seriesLabel}${s2}`);
                  var i;
                  for (i = 0; i < x.length; i++) {
                    x[i].style.opacity = deselectedOpacity;
                  }
                  x = document.getElementsByClassName(`dataPoint${s2}`);
                  for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = deselectedOpacity;
                  }
                }
              }
              
              showValue.innerHTML = `${plt.xAxisLabel} (${plt.xSymbol}): ${(plt.dataX[s][a]).toFixed(1)}&deg;, ${plt.yAxisLabel} (${plt.ySymbol}): ${(plt.dataY[s][a]).toFixed(2)}`;

              var SVGRect = showValue.getBBox();
              rect.setAttribute('width', SVGRect.width);
              rect.setAttribute('height', SVGRect.height);
              rect.setAttribute('y', -plotHeight - 10 - SVGRect.height);

            }, false);
            c.addEventListener('mouseout', function() {
              var s2;
              for (s2 = 0; s2 < numSeries; s2++) {
                if (s2 != s) {
                  var x = document.getElementsByClassName(`${seriesLabel}${s2}`);
                  var i;
                  for (i = 0; i < x.length; i++) {
                    x[i].style.opacity = regularOpacity;
                  }
                  x = document.getElementsByClassName(`dataPoint${s2}`);
                  for (i = 0; i < x.length; i++) {
                      x[i].style.opacity = regularOpacity;
                  }
                }
              }
              showValue.innerHTML = "";

              rect.setAttribute('width', 0);
              rect.setAttribute('height', 0);
            }, false);
            g2.appendChild(c);
          }
        }

        // showValue.setAttribute('y', -plotHeight - 10);
        g.appendChild(showValue);

        var plotBorder = document.createElementNS(svgns, "circle");
        plotBorder.setAttribute('id', 'plotBorder');
        plotBorder.setAttribute('cx', plotWidth/2);
        plotBorder.setAttribute('cy', -plotHeight/2);
        plotBorder.setAttribute('r', plotRadius);
        plotBorder.style.fill = "none";
        // plotBorder.style.stroke = "black";
        g.appendChild(plotBorder);

        // add a legend
        if (plt.seriesLabels.length > 0 && plt.drawLegend) {
          var legend = document.createElementNS(svgns, "g");
          legend.setAttribute('id', 'legend');
          g.appendChild(legend);
          for (let s = 0; s < numSeries; s++) {
            var y = -(plotHeight - 35 - 23 * (s+1));
            var line = document.createElementNS(svgns, "line");
            line.setAttribute('x1', plotWidth + 10);
            line.setAttribute('y1', y);
            line.setAttribute('x2', plotWidth + 40);
            line.setAttribute('y2', y);
            if (plt.drawPoints) {
              line.setAttribute('class', `seriesLine${s}`);
            } else {
              line.setAttribute('class', `seriesLineThick${s}`);
            }
            legend.appendChild(line);

            if (plt.drawPoints) {
              var c = document.createElementNS(svgns, "circle");
              c.setAttribute('cx', plotWidth+25);
              c.setAttribute('cy', y);
              c.setAttribute('r', radius);
              c.setAttribute('class', `dataPoint${s}`);
              legend.appendChild(c);
            }

            var legendLabel = document.createElementNS(svgns, "text");
            legendLabel.setAttribute('x', plotWidth + 48);
            legendLabel.setAttribute('y', y);
            legendLabel.setAttribute('text-anchor', 'left');
            // legendLabel.setAttribute('text-anchor', 'middle');
            legendLabel.setAttribute('dominant-baseline', 'middle');
            legendLabel.setAttribute('class', 'xTickLabel');
            legendLabel.innerHTML = plt.seriesLabels[s];
            legend.appendChild(legendLabel);
          }
        }

        
        // // Add an invisible context menu to the parentId.
        // var cm = document.createElement('div');
        // cm.id = 'context-menu_${plot.parentId}';
        // cm.className = 'context-menu';
        // var list = document.createElement('ul');

        // var copyBtn = document.createElement("li");
        // copyBtn.innerHTML = "Copy";
        // copyBtn.addEventListener('click', function() {
        //   var content = `<svg xmlns='${svgns}' width='${plot.width}' height='${plot.height}'>\n${svg.innerHTML}\n</svg>`;
        //   // Create an auxiliary hidden input
        //   var aux = document.createElement("input");
        //   // Get the text from the element passed into the input
        //   aux.setAttribute("value", content);
        //   // Append the aux input to the body
        //   document.body.appendChild(aux);
        //   // Highlight the content
        //   aux.select();
        //   // Execute the copy command
        //   document.execCommand("copy");
        //   // Remove the input from the body
        //   document.body.removeChild(aux);
        //   // Give a notification
        //   alert("The plot's SVG content has been copied to the clipboard.");
        // }, false);
        // list.appendChild(copyBtn);

        // var gridlineBtn = document.createElement("li");
        // var verb = plot.drawGridlines ? "Hide " : "Show ";
        // gridlineBtn.innerHTML = verb + "Gridlines";
        // gridlineBtn.addEventListener('click', function() {
        //   plot.drawGridlines = !plot.drawGridlines;
        //   // update the context menu label
        //   var verb = plot.drawGridlines ? "Hide " : "Show ";
        //   gridlineBtn.innerHTML = verb + "Gridlines";
        //   update(svg);
        // }, false);
        // list.appendChild(gridlineBtn);

        // if (plot.seriesLabels.length > 0) {
        //   var legendBtn = document.createElement("li");
        //   var verb = plot.drawLegend ? "Hide " : "Show ";
        //   legendBtn.innerHTML = verb + "Legend";
        //   legendBtn.addEventListener('click', function() {
        //     plot.drawLegend = !plot.drawLegend;
        //     // update the context menu label
        //     var verb = plot.drawLegend ? "Hide " : "Show ";
        //     legendBtn.innerHTML = verb + "Legend";
        //     update(svg);
        //   }, false);
        //   list.appendChild(legendBtn);
        // }

        // var pointsBtn = document.createElement("li");
        // var verb = plot.drawPoints ? "Hide " : "Show ";
        // pointsBtn.innerHTML = verb + "Points";
        // pointsBtn.addEventListener('click', function() {
        //   plot.drawPoints = !plot.drawPoints;
        //   // update the context menu label
        //   var verb = plot.drawPoints ? "Hide " : "Show ";
        //   pointsBtn.innerHTML = verb + "Points";
        //   update(svg);
        // }, false);
        // list.appendChild(pointsBtn);

        // var backgroundColorBtn = document.createElement("li");
        // var verb = plot.drawGreyBackground ? "Light " : "Dark ";
        // backgroundColorBtn.innerHTML = verb + "Background";
        // backgroundColorBtn.addEventListener('click', function() {
        //   plot.drawGreyBackground = !plot.drawGreyBackground;
        //   // update the context menu label
        //   var verb = plot.drawGreyBackground ? "Light " : "Dark ";
        //   backgroundColorBtn.innerHTML = verb + "Background";
        //   update(svg);
        // }, false);
        // list.appendChild(backgroundColorBtn);

        // cm.appendChild(list);
        // document.getElementById(plot.parentId).appendChild(cm);

        // var menu = document.getElementById('context-menu_${plot.parentId}');
        // document.onclick = function () {
        //     menu.style.display = 'none';
        // };

        // document.getElementById('plotSvg${plot.parentId}').oncontextmenu = function (evt) {
        //     evt = (evt) ? evt : ((event) ? event : null);
        //     var posnX = (evt.pageX) ? evt.pageX : ((evt.offsetX) ? evt.offsetX + 10 : null);
        //     var posnY = (evt.pageY) ? evt.pageY : ((evt.offsetY) ? evt.offsetY + 10 : null);
        //     menu.style.left = posnX + 'px';
        //     menu.style.top = posnY + 'px';
        //     menu.style.display = 'block';
        //     if (typeof evt.preventDefault != "undefined") {
        //         evt.preventDefault();
        //     } else {
        //         evt.returnValue = false;
        //     }
        // };
      }

      function decimalPlaces(num) {
        var match = (''+num).match(/(?:\.(\d+))?(?:[eE]([+-]?\d+))?$/);
        if (!match) { return 0; }
        return Math.max(
             0,
             // Number of digits right of decimal point.
             (match[1] ? match[1].length : 0)
             // Adjust for scientific notation.
             - (match[2] ? +match[2] : 0));
      }

      update(null);
    </script>"#);

        s
    }
}
