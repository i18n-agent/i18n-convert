class I18nConvert < Formula
  desc "Cross-platform localization file format converter"
  homepage "https://github.com/i18n-agent/i18n-convert"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/i18n-agent/i18n-convert/releases/download/v#{version}/i18n-convert-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/i18n-agent/i18n-convert/releases/download/v#{version}/i18n-convert-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/i18n-agent/i18n-convert/releases/download/v#{version}/i18n-convert-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/i18n-agent/i18n-convert/releases/download/v#{version}/i18n-convert-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "i18n-convert"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/i18n-convert --version")
  end
end
