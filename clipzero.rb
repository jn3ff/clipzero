class Clipzero < Formula
  desc "A simple clipboard manager"
  homepage "https://github.com/jn3ff/clipzero"
  url "https://github.com/jn3ff/clipzero/archive/refs/tags/test.tar.gz"
  version "0.1.0"
  sha256 "afee961455ccc3a98c3e8f285cb11d41d436a07d19f46422104477770d009875"
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
