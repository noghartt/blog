---
title: "Set up macOS with Nix using flakes, nix-darwin and home-manager"
pubDate: 2024-03-03T19:37:49.312Z
tags:
  - nix
---

About three years ago, I started using NixOS on my personal laptop. In those days, I
needed a config that would allow me to have a stable system, that could be reproducible
over all my machines, and that would allow me to have a development environment that would
be easy to maintain and to share with others. NixOS comes to fit all these requirements.

But, since I started using a MacBook Pro (2022), I do not feel the necessity of using the
Nix package manager, since I can use Homebrew to install all the packages that I need and
have some kind of reproducibility with shell scripts. But, for some needs, I would like to
use Nix again on my macOS now, and this blog post will be about how to set up Nix on macOS
like me.

## Installing Nix on macOS

Installing Nix on macOS is easy. You can use the following command to install Nix on your
machine:

```sh
curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install
```

This command installs the Nix package manager based on the [DeterminateSystem/nix-installer](https://github.com/DeterminateSystems/nix-installer),
based on the explanation by the [Zero to Nix](https://zero-to-nix.com/concepts/nix-installer), it gives
better error messages, an installation plan (like Terraform), and other cool features that bring
a better installation experience for you.

Just follow the step by step of the installation flow and everything will be fine.

## Creating the flake file

After installing Nix, you can create a `flake.nix` file that will be the entry point for your
entire Nix configuration on your machine. This file will be used to define the packages that
you want to install, the system configuration, and the home-manager configuration.

```nix
{
  description = "Nix configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";

    nix-darwin.url = "github:lnl7/nix-darwin/master";
    nix-darwin.inputs.nixpkgs.follows = "nixpkgs";

    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ { self, ... }: let
    nixpkgsConfig = {
      config.allowUnfree = true;
    };
  in {

  };
}
```

This is the basic structure of my `flake.nix`, it defines all the inputs that I will need
to use in my configuration, they are:

- `nixpkgs`: The main NixOS/Nixpkgs repository, that will be used to define the packages
  that I want to install on my machine.
- `nix-darwin`: The nix-darwin repository. It brings all modules to configure your macOS
  system using a declarative way.
- `home-manager`: The home-manager repository. It brings all modules to configure your
  user environment using a declarative way.

## Configuring nix-darwin

After creating the `flake.nix` file, you can configure the `nix-darwin` module. You will
need to create a `darwinConfigurations` field on the outputs of your `flake.nix` file, and
add the hostname of your machine with the `darwinSystem` function.

```nix
{
  # ...

  outputs = inputs @ { self, nix-homebrew, home-manager ... }: let
    nixpkgsConfig = {
      config.allowUnfree = true;
    };
  in {
    darwinConfigurations = let
      inherit (inputs.nix-darwin.lib) darwinSystem;
    in {
      machine = darwinSystem {
        system = "aarch64-darwin";

        specialArgs = { inherit inputs; };

        modules = [
          ./hosts/mbp/configuration.nix
          inputs.home-manager.darwinModules.home-manager
          {
            nixpkgs = nixpkgsConfig;

            home-manager.useGlobalPkgs = true;
            home-manager.useUserPackages = true;
            home-manager.users.noghartt = import ./home/home.nix;
          }
        ];
      };
    };
  };
}
```

In that case, what I'm doing is: creating a new configuration called `machine` on the `darwinConfigurations`
systems, and declaring some specific properties of that given system, they are:

- The system of that machine is `aarch64-darwin`, which is the system of the new Apple Silicon
  Macs (MacBook Pro M1).
- The `specialArgs` field is used to pass the inputs to the `darwinSystem` function. All modules
  are functions that accept some arguments, the `specialArgs` field lets you pass new arguments
  for all these imported modules.
- The `modules` field is used to define the modules that will be used to configure the system.
  In that case, I'm using the `configuration.nix` file to define my system configuration, the
  `home-manager` module to configure the user environment, the `homebrew.nix` file to install
  all the packages that I want to install using Homebrew, and the `home.nix` file to configure
  my user environment using home-manager.

## Configuring my system

The `configuration.nix` file is used to define some system configurations related to my
MacBook Pro, like user home dir, some extra options related to Nix binaries, and other system-related configurations that you want to declare. In my case, I wrote the following `configuration.nix`:

```nix
_:

{
  services.nix-daemon.enable = true;

  users.users.noghartt = {
    home = "/Users/noghartt";
  };

  nix.extraOptions = ''
    auto-optimise-store = true
    experimental-features = nix-command flakes
    extra-platforms = x86_64-darwin aarch64-darwin
  '';

  homebrew = {
    enable = true;

    casks = [
      "discord"
      "visual-studio-code"
    ];
  };
}
```

## Configuring the home-manager

The `home.nix` file is used to define the user environment configurations using the home-manager.
In my case, I wrote just a simple `home.nix` file that installs some packages for me. It is
like a `configuration.nix` file but for the user environment.

```nix
{ pkgs, ... }:

{
  home.stateVersion = "23.11";

  home.packages = with pkgs; [
    htop
    curl
    coreutils
    jq
  ];

  programs.zsh = {
    enable = true;

    shellAliases = {
      ls = "ls --color";
    };
  };
}

```

With that configuration, I can install the initial packages that I want to use on my
machine using Nix, and gives me the powerful ability of reproducibility in a declarative way.

## Building and activating the configuration

After creating all the files, you can build and activate the configuration using the following.
You just need to run two specific commands to build and activate the configuration on your
machine:

### 1. Building the configuration

```sh
nix build .#darwinConfigurations.machine.config.system
```

It will build the configuration of your machine targeting the `darwinConfigurations.machine`
outputs. Do not forget of changing the `machine` of the given name of your system configuration.

### 2. Activating the configuration

```sh
./result/sw/bin/darwin-rebuild switch --flake .
```

It will activate the configuration of your machine using the `darwin-rebuild` command. It will
switch the configuration of your machine to the new configuration that you have built.

## Conclusion

In this blog post, I showed how to set up Nix on macOS using flakes, nix-darwin, and home-manager
like I do. I hope that this blog post can help you set up your configuration and let you
use Nix on your machine too.

I really love how the Nix package manager works, and I think that it is a great tool to use
on your machine since it gives you a lot of power to manage your system and your user environment.
You really should give it a try.

If you want to see my configuration, you can check it out on my [repository](https://github.com/noghartt/nixcfg),
it's all open source, feel free to use it as a base for your configuration if you want.

If you have any questions, feel free to ask me on [Twitter](https://twitter.com/noghartt) or send me an [email](mailto:hi@noghartt.dev). I will be glad
to help you with your doubts.

## Resources

- [Some notes on using Nix](https://jvns.ca/blog/2023/02/28/some-notes-on-using-nix/)
- [My first steps with Nix on Mac OSX as Homebrew replacement](https://sandstorm.de/de/blog/post/my-first-steps-with-nix-on-mac-osx-as-homebrew-replacement.html)
- [How I use Nix on macOS](https://blog.6nok.org/how-i-use-nix-on-macos/)
- [Setup nix, nix-darwin, and home-manager from scratch on MacBook M1 Pro](https://gist.github.com/jmatsushita/5c50ef14b4b96cb24ae5268dab613050)
- [Nix Starter Config](https://github.com/Misterio77/nix-starter-configs)
