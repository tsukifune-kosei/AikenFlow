import { spawn } from "node:child_process";
import { constants as fsConstants } from "node:fs";
import * as fs from "node:fs/promises";
import * as os from "node:os";
import * as path from "node:path";

const root = process.cwd();
const pkg = JSON.parse(await fs.readFile(path.join(root, "package.json"), "utf8"));
const outputName = `${pkg.name}-${pkg.version}.vsix`;
const outputPath = path.join(root, outputName);
const staging = await fs.mkdtemp(path.join(os.tmpdir(), "aikenflow-vsix-"));
const packageFiles = [];

try {
  await copyRequiredFile("package.json");
  await copyRequiredFile("README.md");
  await copyRequiredFile("LICENSE.md");
  await copyDirectoryIfExists("out");
  await copyDirectoryIfExists(path.join("media", "webview"));
  await copyDirectoryIfExists("images");
  await copyDirectoryIfExists("bin");

  const manifest = renderManifest(pkg, packageFiles);
  await fs.writeFile(path.join(staging, "extension.vsixmanifest"), manifest, "utf8");

  const contentTypes = renderContentTypes([
    "extension.vsixmanifest",
    "[Content_Types].xml",
    ...packageFiles.map((file) => `extension/${toZipPath(file)}`),
  ]);
  await fs.writeFile(path.join(staging, "[Content_Types].xml"), contentTypes, "utf8");

  await fs.rm(outputPath, { force: true });
  await runZip(outputPath, ["extension.vsixmanifest", "[Content_Types].xml", "extension"]);
  console.log(`Packaged: ${outputPath}`);
} finally {
  await fs.rm(staging, { recursive: true, force: true });
}

async function copyRequiredFile(relativePath) {
  const source = path.join(root, relativePath);
  await assertReadable(source, `Required VSIX file is missing: ${relativePath}`);
  await copyFileToExtension(source, relativePath);
}

async function copyDirectoryIfExists(relativePath) {
  const source = path.join(root, relativePath);
  if (!(await exists(source))) return;

  const entries = await fs.readdir(source, { withFileTypes: true });
  for (const entry of entries) {
    const child = path.join(relativePath, entry.name);
    if (entry.isDirectory()) {
      await copyDirectoryIfExists(child);
    } else if (entry.isFile() && shouldPackageFile(child)) {
      await copyFileToExtension(path.join(root, child), child);
    }
  }
}

function shouldPackageFile(relativePath) {
  return path.extname(relativePath) !== ".map" && path.basename(relativePath) !== ".DS_Store";
}

async function copyFileToExtension(source, relativePath) {
  if (!shouldPackageFile(relativePath)) return;
  const normalized = toZipPath(relativePath);
  packageFiles.push(normalized);
  const target = path.join(staging, "extension", ...normalized.split("/"));
  await fs.mkdir(path.dirname(target), { recursive: true });
  await fs.copyFile(source, target);
}

async function assertReadable(filePath, message) {
  try {
    await fs.access(filePath, fsConstants.R_OK);
  } catch {
    throw new Error(message);
  }
}

async function exists(filePath) {
  try {
    await fs.access(filePath, fsConstants.F_OK);
    return true;
  } catch {
    return false;
  }
}

function renderManifest(packageJson, files) {
  const categories = Array.isArray(packageJson.categories) ? packageJson.categories.join(",") : "";
  const extensionKind = Array.isArray(packageJson.extensionKind)
    ? packageJson.extensionKind.join(",")
    : (packageJson.extensionKind ?? "");
  const engine = packageJson.engines?.vscode ?? "*";
  const repositoryUrl = typeof packageJson.repository?.url === "string" ? packageJson.repository.url : "";
  const assets = [
    `<Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />`,
  ];

  if (files.includes("README.md")) {
    assets.push(`<Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true" />`);
  }
  if (files.includes("LICENSE.md")) {
    assets.push(`<Asset Type="Microsoft.VisualStudio.Services.Content.License" Path="extension/LICENSE.md" Addressable="true" />`);
  }
  if (typeof packageJson.icon === "string" && files.includes(toZipPath(packageJson.icon))) {
    assets.push(`<Asset Type="Microsoft.VisualStudio.Services.Icons.Default" Path="extension/${toZipPath(packageJson.icon)}" Addressable="true" />`);
  }
  const tags = Array.isArray(packageJson.keywords) ? packageJson.keywords.join(",") : "";

  return `<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011" xmlns:d="http://schemas.microsoft.com/developer/vsx-schema-design/2011">
  <Metadata>
    <Identity Language="en-US" Id="${xml(packageJson.name)}" Version="${xml(packageJson.version)}" Publisher="${xml(packageJson.publisher)}" />
    <DisplayName>${xml(packageJson.displayName ?? packageJson.name)}</DisplayName>
    <Description xml:space="preserve">${xml(packageJson.description ?? "")}</Description>
    <Tags>${xml(tags)}</Tags>
    <Categories>${xml(categories)}</Categories>
    <GalleryFlags>Public</GalleryFlags>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="${xml(engine)}" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionDependencies" Value="" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionPack" Value="" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionKind" Value="${xml(extensionKind)}" />
      <Property Id="Microsoft.VisualStudio.Code.LocalizedLanguages" Value="" />
      ${repositoryUrl ? `<Property Id="Microsoft.VisualStudio.Services.Links.Source" Value="${xml(repositoryUrl)}" />` : ""}
      ${repositoryUrl ? `<Property Id="Microsoft.VisualStudio.Services.Links.Getstarted" Value="${xml(repositoryUrl)}" />` : ""}
      ${repositoryUrl ? `<Property Id="Microsoft.VisualStudio.Services.Links.GitHub" Value="${xml(repositoryUrl)}" />` : ""}
      ${repositoryUrl ? `<Property Id="Microsoft.VisualStudio.Services.Links.Support" Value="${xml(repositoryUrl.replace(/\.git$/, "/issues"))}" />` : ""}
      ${repositoryUrl ? `<Property Id="Microsoft.VisualStudio.Services.Links.Learn" Value="${xml(repositoryUrl.replace(/\.git$/, "#readme"))}" />` : ""}
      <Property Id="Microsoft.VisualStudio.Services.GitHubFlavoredMarkdown" Value="true" />
      <Property Id="Microsoft.VisualStudio.Services.Content.Pricing" Value="Free" />
    </Properties>
    ${files.includes("LICENSE.md") ? "<License>extension/LICENSE.md</License>" : ""}
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code"/>
  </Installation>
  <Dependencies/>
  <Assets>
    ${assets.join("\n    ")}
  </Assets>
</PackageManifest>
`;
}

function renderContentTypes(files) {
  const defaults = new Map();
  const overrides = [];

  for (const file of files) {
    const extension = path.posix.extname(file);
    if (extension) {
      defaults.set(extension, contentTypeFor(extension));
    } else {
      overrides.push(`<Override PartName="/${xml(file)}" ContentType="application/octet-stream"/>`);
    }
  }

  return `<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  ${[...defaults.entries()].map(([extension, type]) => `<Default Extension="${xml(extension)}" ContentType="${xml(type)}"/>`).join("\n  ")}
  ${overrides.join("\n  ")}
</Types>
`;
}

function contentTypeFor(extension) {
  switch (extension) {
    case ".css":
      return "text/css";
    case ".html":
      return "text/html";
    case ".js":
      return "application/javascript";
    case ".json":
      return "application/json";
    case ".md":
      return "text/markdown";
    case ".svg":
      return "image/svg+xml";
    case ".png":
      return "image/png";
    case ".vsixmanifest":
    case ".xml":
      return "text/xml";
    default:
      return "application/octet-stream";
  }
}

function runZip(output, entries) {
  return new Promise((resolve, reject) => {
    const child = spawn("zip", ["-X", "-D", "-q", "-r", output, ...entries], {
      cwd: staging,
      shell: false,
      stdio: "inherit",
    });
    child.on("error", (error) => {
      reject(new Error(`Could not run zip. Install zip or use a release environment that provides it. ${error.message}`));
    });
    child.on("close", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`zip exited with code ${code ?? "unknown"}`));
      }
    });
  });
}

function toZipPath(value) {
  return value.split(path.sep).join("/");
}

function xml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}
