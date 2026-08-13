using Microsoft.EntityFrameworkCore.Migrations;

#nullable disable

namespace Spoolbook.Desktop.Migrations
{
    /// <inheritdoc />
    public partial class AddProjectVersionChain : Migration
    {
        /// <inheritdoc />
        protected override void Up(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.AddColumn<bool>(
                name: "IsCurrentVersion",
                table: "Projects",
                type: "INTEGER",
                nullable: false,
                defaultValue: false);

            migrationBuilder.AddColumn<string>(
                name: "MeshHash",
                table: "Projects",
                type: "TEXT",
                nullable: true);

            migrationBuilder.AddColumn<int>(
                name: "PreviousVersionProjectId",
                table: "Projects",
                type: "INTEGER",
                nullable: true);

            migrationBuilder.AddColumn<int>(
                name: "VersionNumber",
                table: "Projects",
                type: "INTEGER",
                nullable: false,
                defaultValue: 0);
        }

        /// <inheritdoc />
        protected override void Down(MigrationBuilder migrationBuilder)
        {
            migrationBuilder.DropColumn(
                name: "IsCurrentVersion",
                table: "Projects");

            migrationBuilder.DropColumn(
                name: "MeshHash",
                table: "Projects");

            migrationBuilder.DropColumn(
                name: "PreviousVersionProjectId",
                table: "Projects");

            migrationBuilder.DropColumn(
                name: "VersionNumber",
                table: "Projects");
        }
    }
}
