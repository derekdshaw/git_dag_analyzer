use crate::object_collection::ObjectContainer;
use crate::report_commits::report_commits;
use crate::report_trees::report_trees;
use crate::report_blobs::report_blobs;

pub fn report_all(container: &ObjectContainer) {
    report_commits(container);
    report_trees(container);
    report_blobs(container);
}