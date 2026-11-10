# frozen_string_literal: true

require "digest"

class Vp < Formula
  # Keep this helper in the formula: Homebrew also loads the copy saved in the keg.
  module Preview
    REGISTRY = "https://registry-bridge.viteplus.dev"

    def self.resolve(ref, platform)
      unless ref.match?(/\A(?:[1-9]\d*|[a-fA-F0-9]{40})\z/)
        raise ArgumentError, "HOMEBREW_VP_PR_VERSION must be a PR number or a full commit SHA"
      end

      sha = ref.downcase
      unless sha.match?(/\A[a-f0-9]{40}\z/)
        headers = Utils::Curl.curl_headers(
          "--max-time", "30", "#{REGISTRY}/voidzero-dev/vite-plus@#{ref}",
          wanted_headers: ["x-commit-key"]
        )
        key = headers.fetch(:responses).last.fetch(:headers).fetch("x-commit-key", "")
        sha = key[/\Avoidzero-dev:vite-plus:([a-f0-9]{40})\z/, 1]
        raise "No published Vite+ preview for PR #{ref}; apply the preview-build label first" unless sha
      end

      preview_version = "0.0.0-commit.#{sha}"
      package = "@voidzero-dev/vite-plus-cli-#{platform}"
      # Build subprocesses must use the same commit and need no metadata network access.
      cache = HOMEBREW_CACHE/"vp-preview/#{sha}-#{platform}.json"
      metadata = if cache.file?
        JSON.parse(cache.read)
      else
        result = Utils::Curl.curl_output(
          "--fail", "--silent", "--show-error", "--max-time", "30",
          "#{REGISTRY}/#{package}/#{preview_version}"
        )
        result.assert_success!
        JSON.parse(result.stdout)
      end
      if metadata["name"] != package || metadata["version"] != preview_version
        raise "Preview metadata does not match #{package}@#{preview_version}"
      end

      dist = metadata.fetch("dist")
      shasum = dist.fetch("shasum")
      download_url = "#{REGISTRY}/tarballs/#{package}/#{preview_version}/#{shasum}.tgz"
      integrity = dist.fetch("integrity")
      if !shasum.match?(/\A[a-f0-9]{40}\z/) || dist["tarball"] != download_url ||
         !integrity.match?(%r{\Asha512-[A-Za-z0-9+/]{86}==\z})
        raise "Preview metadata has an invalid download URL or checksum"
      end

      resource = Resource.new("vp-preview") do
        url download_url
        version preview_version
      end
      # npm publishes SHA-512. Verify it before deriving Homebrew's SHA-256;
      # the normal formula fetch reuses this archive from Homebrew's download cache.
      archive = resource.cached_download
      resource.fetch(verify_download_integrity: false, quiet: true) unless archive.file?
      if "sha512-#{Digest::SHA512.file(archive).base64digest}" != integrity
        resource.clear_cache
        raise "Vite+ preview checksum mismatch"
      end
      unless cache.file?
        cache.dirname.mkpath
        cache.atomic_write(JSON.generate(metadata))
      end
      ENV["HOMEBREW_VP_PR_VERSION"] = sha
      { version: preview_version, url: download_url, sha256: Digest::SHA256.file(archive).hexdigest }
    end
  end
  desc "Unified toolchain for the web"
  homepage "https://viteplus.dev/"
  license "MIT"

  preview_ref = ENV["HOMEBREW_VP_PR_VERSION"].presence

  # BEGIN RELEASE ASSETS
  on_macos do
    on_arm do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v1.5.1/vp-aarch64-apple-darwin.tar.gz"
      sha256 "abb274ef3f6d2abf0b85f19a99a8c363e13752c23b9f322797416b4556cb60e0"
    end
    on_intel do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v1.5.1/vp-x86_64-apple-darwin.tar.gz"
      sha256 "0ab1f4e4a5516da655eb10dae8d1a5c12edbb7628ee8915603cc3cd1d811f4fc"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v1.5.1/vp-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "c58d04a092fb3e2366f63c678a86c65b31ce11ef7d2f0b3c150e230c2e205ebe"
    end
    on_intel do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v1.5.1/vp-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "90c731118c05ae7be169cba5aea366ac8a43d8c0db22991c5b96c8db3065faae"
    end
  end
  # END RELEASE ASSETS

  if preview_ref
    os = OS.mac? ? "darwin" : "linux"
    arch = Hardware::CPU.arm? ? "arm64" : "x64"
    platform = "#{os}-#{arch}#{"-gnu" if OS.linux?}"
    preview = Preview.resolve(preview_ref, platform)
    url preview.fetch(:url)
    version preview.fetch(:version)
    sha256 preview.fetch(:sha256)
  end

  conflicts_with "vite-plus", because: "both install vp, vpr, and vpx"

  def install
    libexec.install "vp", "vp-install-lock.yaml"
    bin.install_symlink libexec/"vp"
    bin.install_symlink "vp" => "vpr"
    bin.install_symlink "vp" => "vpx"
  end

  def caveats
    <<~EOS
      Run vp to install your dependencies and configure your shell.
      First use needs access to the Node.js download service and your npm registry.
      vp and pnpm read ~/.npmrc or NPM_CONFIG_USERCONFIG, including registry credentials.
    EOS
  end

  test do
    ENV["VP_HOME"] = testpath/"vp-home"
    ENV["VP_NODE_MANAGER"] = "yes"
    ENV["VP_PM_MANAGER"] = "yes"
    ENV["VP_SELF_SETUP_NO_MODIFY_PATH"] = "1"
    ENV.prepend_path "PATH", testpath/"vp-home/bin"
    ENV.append_path "PATH", testpath/"vp-home/fallback-bin"
    assert_match version.to_s, shell_output("#{bin}/vp --version")
    assert_match "Homebrew", shell_output("#{bin}/vp env doctor node")
    assert_path_exists testpath/"vp-home/cli-packages/#{version}"
    refute_path_exists testpath/"vp-home/current"
  end
end
