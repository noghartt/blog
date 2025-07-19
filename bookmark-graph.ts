import http from 'node:http';
import bookmarks from './src/pages/bookmarks/_bookmarks.json' assert { type: 'json' };

const getNodesFromBookmarks = () => {
  const nodes = {
    nodes: [] as { id: string; label: string; group: number }[],
    links: [] as { source: string; target: string }[],
  }

  // Add bookmark nodes
  for (const bookmark of bookmarks.data) {
    nodes.nodes.push({
      id: bookmark.id,
      label: bookmark.title,
      group: 1,
    });
  }

  // Create hierarchical tag structure
  const allTags = new Set<string>();
  const tagHierarchy = new Map<string, string[]>(); // tag -> its children
  const tagParents = new Map<string, string>(); // tag -> its parent

  // Collect all original tags and generate hierarchy
  bookmarks.data.forEach(bookmark => {
    if (bookmark.tags && Array.isArray(bookmark.tags)) {
      bookmark.tags.forEach(tag => {
        if (tag && tag.trim() !== '') {
          allTags.add(tag);

          // Generate parent tags from path
          const pathParts = tag.split('/');
          for (let i = 1; i <= pathParts.length; i++) {
            const parentPath = pathParts.slice(0, i).join('/');
            allTags.add(parentPath);

            // Create parent-child relationship
            if (i > 1) {
              const parent = pathParts.slice(0, i - 1).join('/');
              if (!tagHierarchy.has(parent)) {
                tagHierarchy.set(parent, []);
              }
              tagHierarchy.get(parent)!.push(parentPath);
              tagParents.set(parentPath, parent);
            }
          }
        }
      });
    }
  });

  // Add all tag nodes (including generated parent tags) with hierarchy levels
  for (const tag of allTags) {
    const hierarchyLevel = tag.split('/').length;
    nodes.nodes.push({
      id: tag,
      label: tag,
      group: hierarchyLevel + 1, // group 2 for root tags, group 3 for level 1, etc.
    });
  }

  // Create hierarchical links between tags (parent-child relationships)
  tagHierarchy.forEach((children, parent) => {
    children.forEach(child => {
      nodes.links.push({
        source: parent,
        target: child,
      });
    });
  });

  // Create links between bookmarks and tags (including all parent tags)
  for (const bookmark of bookmarks.data) {
    if (bookmark.tags && Array.isArray(bookmark.tags)) {
      for (const tag of bookmark.tags) {
        if (tag && tag.trim() !== '') {
          // Link to the specific tag
          nodes.links.push({
            source: bookmark.id,
            target: tag,
          });

          // Link to all parent tags
          const pathParts = tag.split('/');
          for (let i = 1; i < pathParts.length; i++) {
            const parentTag = pathParts.slice(0, i).join('/');
            nodes.links.push({
              source: bookmark.id,
              target: parentTag,
            });
          }
        }
      }
    }
  }

  return JSON.stringify(nodes);
}


const graphD3ForceScript = `\
import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";

// Dynamic dimensions based on viewport
const WIDTH = window.innerWidth;
const HEIGHT = window.innerHeight;

const color = d3.scaleOrdinal(d3.schemeCategory10);

const data = ${JSON.parse(JSON.stringify(getNodesFromBookmarks()))};
console.log(data);
const nodes = data.nodes.map(d => ({ ...d }));
const links = data.links.map(d => ({ ...d }));

// Calculate node degrees (number of connections)
const nodeDegrees = {};
links.forEach(link => {
  nodeDegrees[link.source] = (nodeDegrees[link.source] || 0) + 1;
  nodeDegrees[link.target] = (nodeDegrees[link.target] || 0) + 1;
});

// Add degree information to nodes and calculate scaled radius
const maxDegree = Math.max(...Object.values(nodeDegrees));
const minRadius = 4;
const maxRadius = 16;

// Create clusters based on shared tags for bookmarks
const clusters = {};
const bookmarkClusters = new Map();

// Find bookmarks that share the most tags
const bookmarks = nodes.filter(n => n.group === 1);
bookmarks.forEach(bookmark => {
  const bookmarkData = data.nodes.find(n => n.id === bookmark.id);
  const bookmarkTags = links
    .filter(l => l.source === bookmark.id)
    .map(l => l.target);

  // Find the most connected tag as cluster center
  let primaryTag = null;
  let maxConnections = 0;

  bookmarkTags.forEach(tag => {
    const tagConnections = links.filter(l => l.target === tag || l.source === tag).length;
    if (tagConnections > maxConnections) {
      maxConnections = tagConnections;
      primaryTag = tag;
    }
  });

  if (primaryTag) {
    if (!clusters[primaryTag]) {
      clusters[primaryTag] = [];
    }
    clusters[primaryTag].push(bookmark);
    bookmarkClusters.set(bookmark.id, primaryTag);
  }
});

nodes.forEach(node => {
  const degree = nodeDegrees[node.id] || 1;
  // Scale radius based on degree with a square root to avoid too extreme differences
  const scaleFactor = Math.sqrt(degree / maxDegree);
  node.degree = degree;

  // Different base sizes for different hierarchy levels
  let baseRadius;
  if (node.group === 1) baseRadius = minRadius; // Bookmarks - smallest
  else if (node.group === 2) baseRadius = minRadius + 4; // Root tags - larger
  else if (node.group === 3) baseRadius = minRadius + 2; // Level 1 tags - medium
  else baseRadius = minRadius + 1; // Deeper tags - slightly larger than bookmarks

  node.radius = baseRadius + (maxRadius - baseRadius) * scaleFactor;

  // Add cluster information for bookmarks
  if (node.group === 1) {
    node.cluster = bookmarkClusters.get(node.id) || 'default';
  }
});

// Clustering force function
function forceCluster() {
  const strength = 0.2;
  let nodes;

  function force(alpha) {
    const clusters = {};

    // Group nodes by cluster
    nodes.forEach(d => {
      if (d.group === 1 && d.cluster) {
        if (!clusters[d.cluster]) clusters[d.cluster] = [];
        clusters[d.cluster].push(d);
      }
    });

    // Move nodes toward cluster centroid
    Object.values(clusters).forEach(cluster => {
      if (cluster.length <= 1) return;

      const centroidX = cluster.reduce((sum, d) => sum + d.x, 0) / cluster.length;
      const centroidY = cluster.reduce((sum, d) => sum + d.y, 0) / cluster.length;

      cluster.forEach(d => {
        d.vx += (centroidX - d.x) * strength * alpha;
        d.vy += (centroidY - d.y) * strength * alpha;
      });
    });
  }

  force.initialize = function(_) {
    nodes = _;
  };

  return force;
}

const simulation = d3.forceSimulation(nodes)
  .force('link', d3.forceLink(links)
    .id((d) => d.id)
    .distance(60) // Increased distance for better separation
    .strength(0.4) // Reduced strength to allow clustering
  )
  .force('charge', d3.forceManyBody()
    .strength((d) => {
      if (d.group === 1) return -35000; // Bookmarks
      if (d.group === 2) return -15000; // Root tags (strongest for main categories)
      if (d.group === 3) return -12000; // Level 1 tags
      return -8000; // Deeper level tags (less repulsion for subtags)
    })
    .distanceMin(20)
    .distanceMax(400)
  )
  .force('center', d3.forceCenter(WIDTH / 2, HEIGHT / 2)
    .strength(0.05)
  )
  .force('collision', d3.forceCollide()
    .radius((d) => d.radius + 4) // Increased padding to prevent overlap
    .strength(1.0) // Maximum collision strength
  )
  .force('cluster', forceCluster()) // Add clustering force
  .force('x', d3.forceX(WIDTH / 2)
    .strength(0.01)
  )
  .force('y', d3.forceY(HEIGHT / 2)
    .strength(0.01)
  )
  .alphaDecay(0.015) // Slower decay for better settling
  .velocityDecay(0.5) // Higher damping for stability
  .on('tick', ticked);

const svg = d3.create("svg")
  .attr("width", WIDTH)
  .attr("height", HEIGHT)
  .attr("viewBox", [0, 0, WIDTH, HEIGHT])
  .attr("style", "max-width: 100%; height: auto;");

// Create a group that will be transformed during zoom
const g = svg.append("g");

// Define zoom behavior
const zoom = d3.zoom()
  .scaleExtent([0.1, 10])
  .on("zoom", (event) => {
    g.attr("transform", event.transform);
  });

// Apply zoom to the SVG
svg.call(zoom);

// Add links
const link = g.append("g")
  .attr("stroke", "#999")
  .attr("stroke-opacity", 0.6)
  .selectAll()
  .data(links)
  .join("line")
  .attr("stroke-width", 1);

const node = g.append("g")
  .attr("stroke", "#fff")
  .attr("stroke-width", 1.5)
  .selectAll()
  .data(nodes)
  .join("circle")
  .attr("r", d => d.radius) // Use calculated dynamic radius based on connections
  .attr("fill", d => color(d.group));

node.append("title")
  .text(d => d.label + "\\nConnections: " + d.degree);

// Add text labels first
const labels = g.append("g")
  .selectAll()
  .data(nodes)
  .join("text")
  .text(d => d.label)
  .attr("font-size", "10px")
  .attr("font-family", "Arial, sans-serif")
  .attr("text-anchor", "middle")
  .attr("fill", "#333")
  .attr("dy", "1.5em"); // Position below the node

// No drag functionality - focus on click highlighting only

// State management for persistent highlighting
let highlightedNode = null;
let connectedNodes = new Set();

// Function to apply highlighting
function applyHighlight(selectedNode) {
  highlightedNode = selectedNode;
  connectedNodes.clear();
  connectedNodes.add(selectedNode.id);

  links.forEach(link => {
    if (link.source.id === selectedNode.id) {
      connectedNodes.add(link.target.id);
    } else if (link.target.id === selectedNode.id) {
      connectedNodes.add(link.source.id);
    }
  });

  // Apply highlighting styles
  node
    .style("opacity", n => connectedNodes.has(n.id) ? 1 : 0.2)
    .style("stroke-width", n => n.id === selectedNode.id ? 3 : 1.5);

  link
    .style("opacity", l =>
      (connectedNodes.has(l.source.id) && connectedNodes.has(l.target.id)) ? 1 : 0.1
    )
    .style("stroke-width", l =>
      (l.source.id === selectedNode.id || l.target.id === selectedNode.id) ? 2 : 1
    );

  labels
    .style("opacity", n => connectedNodes.has(n.id) ? 1 : 0.2)
    .style("font-weight", n => n.id === selectedNode.id ? "bold" : "normal");
}

// Function to clear highlighting
function clearHighlight() {
  highlightedNode = null;
  connectedNodes.clear();

  // Restore original appearance
  node
    .style("opacity", 1)
    .style("stroke-width", 1.5);

  link
    .style("opacity", 0.6)
    .style("stroke-width", 1);

  labels
    .style("opacity", 1)
    .style("font-weight", "normal");
}

// Add click handler for persistent highlighting
node
  .on("click", function(event, d) {
    event.stopPropagation(); // Prevent SVG click event
    applyHighlight(d);
  });

// Add click handler to SVG background to clear highlighting
svg.on("click", function(event) {
  clearHighlight();
});

// Add ESC key handler to clear highlighting
document.addEventListener("keydown", function(event) {
  if (event.key === "Escape") {
    clearHighlight();
  }
});

function ticked() {
  link
    .attr("x1", d => d.source.x)
    .attr("y1", d => d.source.y)
    .attr("x2", d => d.target.x)
    .attr("y2", d => d.target.y);

  node
    .attr("cx", d => d.x)
    .attr("cy", d => d.y);

  labels
    .attr("x", d => d.x)
    .attr("y", d => d.y);
}


// Handle window resize
function handleResize() {
  const newWidth = window.innerWidth;
  const newHeight = window.innerHeight;

  // Update SVG dimensions
  svg
    .attr("width", newWidth)
    .attr("height", newHeight)
    .attr("viewBox", [0, 0, newWidth, newHeight]);

  // Update force simulation center
  simulation
    .force('center', d3.forceCenter(newWidth / 2, newHeight / 2).strength(0.1))
    .force('x', d3.forceX(newWidth / 2).strength(0.02))
    .force('y', d3.forceY(newHeight / 2).strength(0.02))
    .alpha(0.3)
    .restart();
}

// Add resize event listener
window.addEventListener('resize', handleResize);

const container = document.getElementById("container");
container.appendChild(svg.node());
`;

const generateHtml = () => {
  const html = `\
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Bookmark Graph</title>
    <style>
      * {
        margin: 0;
        padding: 0;
        box-sizing: border-box;
      }

      html, body {
        width: 100%;
        height: 100%;
        overflow: hidden;
      }

      #container {
        width: 100vw;
        height: 100vh;
        background-color: #f9f9f9;
      }

      svg {
        width: 100%;
        height: 100%;
        display: block;
      }
    </style>
  </head>
  <body>
    <div id="container"></div>
    <script type="module">
      ${graphD3ForceScript}
    </script>
  </body>
</html>
  `;

  return html;
}

const createServer = async () => {
  const server = http.createServer((req, res) => {
    if (req.url !== '/') {
      res.writeHead(404, { 'Content-Type': 'text/plain' });
      res.end('Not found');
      return;
    }

    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(generateHtml());
  });
  server.listen(3000, () => {
    console.log('Server is running on port 3000');
  });
};

createServer();
