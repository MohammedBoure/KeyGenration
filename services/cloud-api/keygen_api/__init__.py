from flask import Flask

from .api import api
from .config import load_environment
from .dashboard import dashboard


def create_app():
    load_environment()
    application = Flask(__name__, template_folder="templates")
    application.register_blueprint(api)
    application.register_blueprint(dashboard)
    return application
