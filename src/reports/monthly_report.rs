use chrono::{DateTime, Utc, Datelike, Duration};
use printpdf::*;
use std::collections::HashMap;
use std::io::BufWriter;
use sqlx::PgPool;

use crate::repos::{
    expense_entry::ExpenseEntryRepo,
    category::CategoryRepo,
    budget::BudgetRepo,
};

#[derive(Debug)]
pub struct MonthlyExpenseData {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_expenses: f64,
    pub category_breakdown: HashMap<String, f64>,
    pub budget_comparison: HashMap<String, BudgetComparison>,
    pub previous_month_total: f64,
    pub expense_trend: Vec<(String, f64)>, // Last 6 months
}

#[derive(Debug)]
pub struct BudgetComparison {
    pub budget_amount: f64,
    pub spent_amount: f64,
    pub remaining: f64,
    pub percentage_used: f64,
    pub status: BudgetStatus,
}

#[derive(Debug)]
pub enum BudgetStatus {
    OnTrack,
    NearLimit,
    OverBudget,
}

#[derive(Clone)]
pub struct MonthlyReportGenerator {
    db_pool: PgPool,
}

impl MonthlyReportGenerator {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn generate_monthly_report(
        &self,
        group_uid: uuid::Uuid,
        user_uid: uuid::Uuid,
        start_over_date: i16,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Calculate current month period
        let (current_start, current_end) = self.calculate_month_range(start_over_date);

        // Gather all data
        let expense_data = self.gather_expense_data(
            group_uid,
            user_uid,
            current_start,
            current_end,
        ).await?;

        // Generate PDF
        let pdf_bytes = self.create_pdf_report(expense_data).await?;

        Ok(pdf_bytes)
    }

    async fn gather_expense_data(
        &self,
        group_uid: uuid::Uuid,
        user_uid: uuid::Uuid,
        current_start: DateTime<Utc>,
        current_end: DateTime<Utc>,
    ) -> Result<MonthlyExpenseData, Box<dyn std::error::Error + Send + Sync>> {
        let mut tx = self.db_pool.begin().await?;

        // Get current month expenses
        let current_expenses = ExpenseEntryRepo::list_by_group(&mut tx, group_uid).await?;
        let mut category_breakdown = HashMap::new();
        let mut total_expenses = 0.0;

        for expense in current_expenses {
            if expense.created_by == user_uid.to_string()
                && expense.created_at >= current_start
                && expense.created_at < current_end {

                let category = CategoryRepo::get(&mut tx, expense.category_uid).await?;
                let category_name = category.name;

                *category_breakdown.entry(category_name).or_insert(0.0) += expense.price;
                total_expenses += expense.price;
            }
        }

        // Get budget information
        let budgets = BudgetRepo::list_by_group(&mut tx, group_uid).await?;
        let mut budget_comparison = HashMap::new();

        for budget in budgets {
            let category = CategoryRepo::get(&mut tx, budget.category_uid).await?;
            let spent = category_breakdown.get(&category.name).unwrap_or(&0.0);
            let remaining = budget.amount - spent;
            let percentage = if budget.amount > 0.0 { (spent / budget.amount) * 100.0 } else { 0.0 };

            let status = if remaining < 0.0 {
                BudgetStatus::OverBudget
            } else if percentage >= 80.0 {
                BudgetStatus::NearLimit
            } else {
                BudgetStatus::OnTrack
            };

            budget_comparison.insert(category.name, BudgetComparison {
                budget_amount: budget.amount,
                spent_amount: *spent,
                remaining,
                percentage_used: percentage,
                status,
            });
        }

        // Get previous month total
        let previous_month_start = current_start - Duration::days(30);
        let previous_month_end = current_start;

        let previous_expenses = ExpenseEntryRepo::list_by_group(&mut tx, group_uid).await?;
        let mut previous_total = 0.0;

        for expense in previous_expenses {
            if expense.created_by == user_uid.to_string()
                && expense.created_at >= previous_month_start
                && expense.created_at < previous_month_end {
                previous_total += expense.price;
            }
        }

        // Get expense trend (last 6 months)
        let mut expense_trend = Vec::new();
        for i in (0..6).rev() {
            let month_start = current_start - Duration::days(30 * i);
            let month_end = month_start + Duration::days(30);

            let month_expenses = ExpenseEntryRepo::list_by_group(&mut tx, group_uid).await?;
            let mut month_total = 0.0;

            for expense in month_expenses {
                if expense.created_by == user_uid.to_string()
                    && expense.created_at >= month_start
                    && expense.created_at < month_end {
                    month_total += expense.price;
                }
            }

            let month_name = format!("{} {}", month_start.format("%B"), month_start.year());
            expense_trend.push((month_name, month_total));
        }

        tx.commit().await?;

        Ok(MonthlyExpenseData {
            period_start: current_start,
            period_end: current_end,
            total_expenses,
            category_breakdown,
            budget_comparison,
            previous_month_total: previous_total,
            expense_trend,
        })
    }

    async fn create_pdf_report(
        &self,
        data: MonthlyExpenseData,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Create PDF document
        let (doc, page1, layer1) = PdfDocument::new(
            "Monthly Expense Report",
            Mm(210.0), // A4 width
            Mm(297.0), // A4 height
            "Layer 1"
        );

        let current_layer = doc.get_page(page1).get_layer(layer1);

        // Add title
        let font = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
        current_layer.use_text(
            format!("Monthly Expense Report - {}", data.period_start.format("%B %Y")),
            24.0,
            Mm(20.0),
            Mm(280.0),
            &font
        );

        // Add summary section
        let font_regular = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        let mut y_position = 250.0;

        current_layer.use_text(
            "Summary",
            18.0,
            Mm(20.0),
            Mm(y_position),
            &font
        );
        y_position -= 15.0;

        current_layer.use_text(
            &format!("Total Expenses: Rp. {:.0}", data.total_expenses),
            12.0,
            Mm(25.0),
            Mm(y_position),
            &font_regular
        );
        y_position -= 10.0;

        let change_percentage = if data.previous_month_total > 0.0 {
            ((data.total_expenses - data.previous_month_total) / data.previous_month_total) * 100.0
        } else {
            0.0
        };

        let change_text = if change_percentage > 0.0 {
            format!("↗️ +{:.1}% from last month", change_percentage)
        } else if change_percentage < 0.0 {
            format!("↘️ {:.1}% from last month", change_percentage)
        } else {
            "→ No change from last month".to_string()
        };

        current_layer.use_text(
            &change_text,
            12.0,
            Mm(25.0),
            Mm(y_position),
            &font_regular
        );
        y_position -= 20.0;

        // Add category breakdown
        current_layer.use_text(
            "Category Breakdown",
            16.0,
            Mm(20.0),
            Mm(y_position),
            &font
        );
        y_position -= 15.0;

        for (category, amount) in &data.category_breakdown {
            let percentage = if data.total_expenses > 0.0 {
                (amount / data.total_expenses) * 100.0
            } else {
                0.0
            };

            current_layer.use_text(
                &format!("{}: Rp. {:.0} ({:.1}%)", category, amount, percentage),
                12.0,
                Mm(25.0),
                Mm(y_position),
                &font_regular
            );
            y_position -= 10.0;
        }

        y_position -= 10.0;

        // Add budget comparison
        if !data.budget_comparison.is_empty() {
            current_layer.use_text(
                "Budget Status",
                16.0,
                Mm(20.0),
                Mm(y_position),
                &font
            );
            y_position -= 15.0;

            for (category, budget) in &data.budget_comparison {
                let status_text = match budget.status {
                    BudgetStatus::OnTrack => "✅ On track",
                    BudgetStatus::NearLimit => "⚠️ Near limit",
                    BudgetStatus::OverBudget => "❌ Over budget",
                };

                current_layer.use_text(
                    &format!("{}: Rp. {:.0}/Rp. {:.0} ({:.1}%) {}",
                        category, budget.spent_amount, budget.budget_amount,
                        budget.percentage_used, status_text),
                    12.0,
                    Mm(25.0),
                    Mm(y_position),
                    &font_regular
                );
                y_position -= 10.0;
            }
        }

        // Generate and add chart
        if y_position > 100.0 {
            let _chart_image = self.generate_expense_chart(&data.expense_trend)?;
            // Note: In a real implementation, you'd embed the chart image in the PDF
            // This is a simplified version
        }

        // Save PDF to bytes
        let mut bytes = Vec::new();
        {
            let mut writer = BufWriter::new(&mut bytes);
            doc.save(&mut writer)?;
        } // writer goes out of scope here, releasing the borrow

        Ok(bytes)
    }

    fn generate_expense_chart(
        &self,
        _expense_trend: &[(String, f64)],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Simplified chart generation - in a real implementation,
        // you'd use a proper bitmap backend or external service
        // For now, return empty bytes
        Ok(Vec::new())
    }

    fn calculate_month_range(&self, start_over_date: i16) -> (DateTime<Utc>, DateTime<Utc>) {
        let now = Utc::now();
        let current_year = now.year();
        let current_month = now.month();

        // Calculate the start date based on start_over_date
        let start_day = start_over_date as u32;
        let mut start_date = if current_month == 1 {
            // January - go back to previous year
            chrono::NaiveDate::from_ymd_opt(current_year - 1, 12, start_day)
        } else {
            chrono::NaiveDate::from_ymd_opt(current_year, current_month - 1, start_day)
        }.unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(current_year, current_month, 1).unwrap());

        // If the calculated start date is in the future, use the previous month's start date
        if start_date > now.date_naive() {
            start_date = if current_month == 1 {
                chrono::NaiveDate::from_ymd_opt(current_year - 1, 11, start_day)
            } else if current_month == 2 {
                chrono::NaiveDate::from_ymd_opt(current_year - 1, 12, start_day)
            } else {
                chrono::NaiveDate::from_ymd_opt(current_year, current_month - 2, start_day)
            }.unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(current_year, current_month - 1, 1).unwrap());
        }

        let end_date = start_date + Duration::days(30); // Approximate month length

        (
            start_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            end_date.and_hms_opt(23, 59, 59).unwrap().and_utc(),
        )
    }
}