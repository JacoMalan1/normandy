{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }: 
  let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
  in
  {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          openssl
        ];

        env = {
          OPENSSL_DIR = "${pkgs.openssl.out}";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
        };
      };
  };
}
