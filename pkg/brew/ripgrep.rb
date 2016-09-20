require  'formula'
class Ripgrep < Formula
  version '0.1.17'
  desc "Search tool like grep and The Silver Searcher."
  homepage "https://github.com/BurntSushi/ripgrep"

  if Hardware::CPU.is_64_bit?
    url "https://github.com/BurntSushi/ripgrep/releases/download/#{version}/ripgrep-#{version}-x86_64-apple-darwin.tar.gz"
    sha256 "cb7b551a08849cef6ef8f17229224f094299692981976a3c5873c93f68c8fa1a"
  else
    url "https://github.com/BurntSushi/ripgrep/releases/download/#{version}/ripgrep-#{version}-i686-apple-darwin.tar.gz"
    sha256 "0e936874b9f3fd661c5566e7f8fe18343baa5e9371e57d8d71000e9234fc376b"
  end

  def install
    bin.install "rg"
    man1.install "rg.1"
  end
end
