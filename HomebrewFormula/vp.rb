# frozen_string_literal: true

class Vp < Formula
  desc "Unified toolchain for the web"
  homepage "https://viteplus.dev/"
  license "MIT"

  # The updater removes this gate after the first compatible release is published.
  disable! date: "2026-09-22", because: "requires a release with per-user Homebrew setup"

  # BEGIN RELEASE ASSETS
  on_macos do
    on_arm do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v0.3.3/vp-aarch64-apple-darwin.tar.gz"
      sha256 "856713a318f1cc3b33c9dface7ca2ef066f18ede8150f37b7f549060d4709834"
    end
    on_intel do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v0.3.3/vp-x86_64-apple-darwin.tar.gz"
      sha256 "75095001972a6974ea1943d25e28dc47bd7a7f77f8387055e70d86a499a778c2"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v0.3.3/vp-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "b3cafb9545623859910ce06a905da2d3d577fe4c15558795b8ddd09e1fbe9ed1"
    end
    on_intel do
      url "https://github.com/voidzero-dev/vite-plus/releases/download/v0.3.3/vp-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0acf34e3182d20f245b58fb0a4c611329b040f64d9d2b76b91bbd907bc9fc4f8"
    end
  end
  # END RELEASE ASSETS

  conflicts_with "vite-plus", because: "both install vp, vpr, and vpx"

  def install
    bin.install "vp"
    bin.install_symlink "vp" => "vpr"
    bin.install_symlink "vp" => "vpx"
  end

  def caveats
    <<~EOS
      Run vp to install your dependencies and configure your shell.
      First use needs access to the Node.js download service and your npm registry.
      npm and pnpm read ~/.npmrc or NPM_CONFIG_USERCONFIG, including registry credentials.
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
