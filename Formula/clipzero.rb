class Clipzero < Formula
  desc "A simple clipboard manager"
  homepage "https://github.com/jn3ff/clipzero"
  url "https://github.com/jn3ff/clipzero/archive/refs/tags/v0.1.1.tar.gz"
  version "0.1.1"
  sha256 "ee12b2f308e2ee177d2df0cde2c90b5c6467bd251366cdd26016cd08f69f2bca"
  license "MIT"

  depends_on "rust" => :build

  service do
    run [opt_bin/"clipzero"]
    keep_alive true
    working_dir var
    log_path var/"log/clipzero.log"
    error_log_path var/"log/clipzero.error.log"
  end

  def install
    system "cargo", "install", "--root", prefix, "--path", "."
  end

  test do
    assert_match "version", shell_output("#{bin}/clipzero --version")
  end
end
