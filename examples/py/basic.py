# Same sintax as rust

import pypnir

def main():
    exe_path = "../../build/test"

    sandbox = (
        pypnir.Sandbox.setup()
        .path(exe_path)
        .build()
    )

    print(f"Path sandbox: {sandbox.path()}")
    sandbox.run()

if __name__ == "__main__":
    main()
