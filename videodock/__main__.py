import sys


def main():
    if '--worker' in sys.argv:
        from videodock.worker import run
        return run()
    if '--version' in sys.argv:
        from videodock import __version__
        print(__version__)
        return 0
    from videodock.ui import launch
    return launch()


if __name__ == '__main__':
    raise SystemExit(main())
