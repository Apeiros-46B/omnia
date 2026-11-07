{
	description = "Omnia development environment";

	inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

	outputs = { nixpkgs, ... }:
		let
			systems = [
				"x86_64-linux"
				"aarch64-linux"
				"x86_64-darwin"
				"aarch64-darwin"
			];
		in {
			devShells = nixpkgs.lib.genAttrs systems (system:
				let
					pkgs = nixpkgs.legacyPackages.${system};
				in {
					default = pkgs.mkShell {
						packages = with pkgs; [
							just
							cargo
							pnpm

							rustc
							rust-analyzer
							nodejs_24
							typescript-language-server
							svelte-language-server

							sqlx-cli
						];
						shellHook = ''
							export DATABASE_URL="sqlite:$(realpath .)/apps/server/test.db"
						'';
					};
				}
			);
		};
}
