from flask import Blueprint, render_template


dashboard = Blueprint("dashboard", __name__)


@dashboard.get("/dashboard")
def admin_dashboard():
    return render_template("dashboard.html")
